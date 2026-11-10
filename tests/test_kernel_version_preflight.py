import hashlib
import os
from pathlib import Path
import shlex
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/resukisu-kernel-version"
PACKAGER = ROOT / "tools/package-kernel-version-probe.py"

DEFAULT_INFO = """version: 35240
full_version: v4.2.0-rc3-85-gabc1234@BakaSU
flags: 0x1f
uapi_version: 6
features: 0x3
lkm: true
bundled: false
late_load: false
runtime_mode: lkm
pr_build: false
"""
DEFAULT_VERSION = "Kernel Version: 35240\n"
DEFAULT_DMESG = """[    0.000000] unrelated boot line
[    0.100000] KernelSU: Initialized with driver version: 35240, full_version: v4.2.0-rc3-85-gabc1234@BakaSU, ABI: aarch64, Work mode: LKM, ksu_uapi_version: 6
[   13.000000] handle_toolkit_cmd: ksu_change_ksuver to: 32704
[   27.000000] handle_toolkit_cmd: ksu_change_ksuver to: 35240
[   27.100000] unrelated secret line
"""
DEFAULT_CONFIG = """CONFIG_KSU=y
# CONFIG_KSU_TOOLKIT_SUPPORT is not set
CONFIG_KSU_SUSFS=y
CONFIG_KSU_MULTI_MANAGER_SUPPORT=y
CONFIG_UNRELATED=y
"""


class KernelVersionPreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.case = self.root / "module"
        shutil.copytree(FIXTURE, self.case)

        self.manager = self.case / "manager"
        self.manager.write_text(
            '#!/bin/sh\n'
            'printf "%s\\n" "$*" >> .manager_calls\n'
            'case "$*" in\n'
            '    "debug info") cat "$TEST_INFO_FILE" ;;\n'
            '    "debug version") cat "$TEST_VERSION_FILE" ;;\n'
            '    *) exit 98 ;;\n'
            'esac\n'
        )
        self.manager.chmod(0o755)
        self.info_file = self.case / "info_output"
        self.version_file = self.case / "version_output"
        self.info_file.write_text(DEFAULT_INFO)
        self.version_file.write_text(DEFAULT_VERSION)

        self.context_input = self.case / "context_input"
        self.context_input.write_text("u:r:ksu:s0")
        self.uptime_input = self.case / "uptime_input"
        self.uptime_input.write_text("13.60 678.90\n")
        self.dmesg_input = self.case / "dmesg_input"
        self.dmesg_input.write_text(DEFAULT_DMESG)
        self.config_input = self.case / "config_input"
        self.config_input.write_text(DEFAULT_CONFIG)

        self.modules_root = self.root / "modules"
        (self.modules_root / "alpha").mkdir(parents=True)
        (self.modules_root / "stray-file").write_text("not a module\n")
        toolkit = self.modules_root / "ksu_toolkit"
        toolkit.mkdir()
        self.toolkit_marker = self.case / "toolkit-ran"
        (toolkit / "toolkit").write_text('#!/bin/sh\ntouch "$TEST_TOOLKIT_MARKER"\n')
        (toolkit / "system" / "bin").mkdir(parents=True)
        (toolkit / "system" / "bin" / "toolkit").write_text(
            '#!/bin/sh\ntouch "$TEST_TOOLKIT_MARKER"\n'
        )

        self.manager_version_path = self.case / "manager_version"
        self.manager_version_path.write_text("35240\n")

        self.busybox = self.case / "busybox"
        self.busybox.write_text(
            '#!/bin/sh\n'
            'if [ "$1" = zcat ]; then\n'
            '    cat "$TEST_CONFIG_FILE"\n'
            'else\n'
            '    exec "$@"\n'
            'fi\n'
        )
        self.busybox.chmod(0o755)

        self.ns_root = self.root / "ns"
        self.ns_root.mkdir()
        (self.ns_root / "mnt").symlink_to("mnt:[4026533853]")
        (self.ns_root / "uts").symlink_to("uts:[4026531838]")

        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.mock("id", '[ "$1" = -u ] && printf "%s\\n" "${TEST_UID:-0}"')
        real_stat = shlex.quote(shutil.which("stat"))
        self.mock(
            "stat",
            f'''if [ "${{3:-}}" = "$TEST_MANAGER_PATH" ]; then
    printf "%s\\n" "${{TEST_MANAGER_STAT:-0:0:755}}"
elif [ "${{3:-}}" = "$TEST_MANAGER_VERSION_PATH" ]; then
    case "${{2:-}}" in
        '%u:%g') printf "%s\\n" "${{TEST_MV_OWNER:-0:0}}" ;;
        '%a') printf "%s\\n" "${{TEST_MV_MODE:-600}}" ;;
        *) exec {real_stat} "$@" ;;
    esac
else
    exec {real_stat} "$@"
fi''',
        )
        self.mock(
            "timeout",
            '''[ "$1" = -s ] && [ "$2" = KILL ] || exit 96
case "$3" in ''|*[!0123456789]*) exit 96 ;; esac
shift 3
case "$1" in
    /system/bin/readlink)
        shift
        exec readlink "$@"
        ;;
    /system/bin/sha256sum)
        shift
        if [ "$1" = "$TEST_MANAGER_PATH" ]; then
            printf '%s  %s\\n' "${TEST_MANAGER_SHA:-fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50}" "$1"
            exit 0
        fi
        exit 96
        ;;
    "$TEST_MANAGER_PATH")
        if [ "${2:-}" = debug ] && [ "${3:-}" = info ] && [ "${TEST_INFO_TIMEOUT:-0}" = 1 ]; then
            exit 137
        fi
        if [ "${2:-}" = debug ] && [ "${3:-}" = version ] && [ "${TEST_VERSION_TIMEOUT:-0}" = 1 ]; then
            exit 137
        fi
        exec "$@"
        ;;
    "$TEST_BUSYBOX_PATH"|dmesg)
        exec "$@"
        ;;
    *) exit 96 ;;
esac''',
        )
        self.mock(
            "sleep",
            '''if [ "${TEST_SLEEP_JUMP:-0}" = 1 ]; then
    printf "200.00 0.00\\n" > "$TEST_UPTIME"
fi
exit 0''',
        )
        self.mock("dmesg", 'cat "$TEST_DMESG_FILE"')

        hook = self.case / "service.sh"
        text = hook.read_text()
        text = text.replace("/data/adb/ksud", str(self.manager))
        text = text.replace("/data/adb/ksu/bin/busybox", str(self.busybox))
        text = text.replace("/data/adb/ksu/.manager_version", str(self.manager_version_path))
        text = text.replace("/data/adb/modules", str(self.modules_root))
        text = text.replace("/proc/uptime", shlex.quote(str(self.uptime_input)))
        text = text.replace("/proc/modules", shlex.quote(str(self.modules_root.parent / "proc_modules")))
        text = text.replace("/proc/config.gz", shlex.quote(str(self.config_input)))
        text = text.replace('"/proc/$$/attr/current"', shlex.quote(str(self.context_input)))
        text = text.replace("/proc/$$/ns", str(self.ns_root))
        hook.write_text(text)

        boot_hook = self.case / "boot-completed.sh"
        boot_hook.write_text(
            boot_hook.read_text().replace("/data/adb/ksu/bin/busybox", str(self.busybox))
        )

        self.proc_modules = self.modules_root.parent / "proc_modules"
        self.proc_modules.write_text(
            "alpha 16384 1 - Live 0x0000000000000000\n"
            "kernelsu 32768 1 - Live 0x0000000000000000\n"
            "unrelated 4096 0 - Live 0x0000000000000000\n"
        )

        self.env = os.environ | {
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "KSU": "true",
            "BAKASU": "true",
            "KSU_MODULE": "earkey_control_kver",
            "KSU_VER_CODE": "35257",
            "KSU_KERNEL_VER_CODE": "35240",
            "KSU_UAPI_VER": "6",
            "KSU_RUNTIME_MODE": "lkm",
            "TEST_MANAGER_PATH": str(self.manager),
            "TEST_MANAGER_VERSION_PATH": str(self.manager_version_path),
            "TEST_BUSYBOX_PATH": str(self.busybox),
            "TEST_INFO_FILE": str(self.info_file),
            "TEST_VERSION_FILE": str(self.version_file),
            "TEST_DMESG_FILE": str(self.dmesg_input),
            "TEST_CONFIG_FILE": str(self.config_input),
            "TEST_UPTIME": str(self.uptime_input),
            "TEST_TOOLKIT_MARKER": str(self.toolkit_marker),
        }

    def mock(self, name, body):
        path = self.bin / name
        path.write_text(f"#!/bin/sh\n{body}\n")
        path.chmod(0o700)

    def run_hook(self, *, stage="service", **changes):
        env = self.env | {key: value for key, value in changes.items() if value is not None}
        for key, value in changes.items():
            if value is None:
                env.pop(key, None)
        return subprocess.run(
            ["dash", str(self.case / ("boot-completed.sh" if stage == "boot_completed" else "service.sh"))],
            env=env, capture_output=True, text=True, timeout=20,
        )

    def record(self, stage="service"):
        name = ".kver-boot" if stage == "boot_completed" else ".kver-service"
        return (self.case / name / "record").read_text()

    def clear_attempt(self, stage="service"):
        name = ".kver-boot" if stage == "boot_completed" else ".kver-service"
        shutil.rmtree(self.case / name, ignore_errors=True)

    def manager_calls(self):
        path = self.case / ".manager_calls"
        return path.read_text().splitlines() if path.exists() else []

    def test_service_samples_and_records_only_structured_fields(self):
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("hook=entered\nstage=service\n", record)
        self.assertIn("uid=0\n", record)
        self.assertIn("env_kernel_version=35240\n", record)
        self.assertIn("selinux_context=u:r:ksu:s0\n", record)
        self.assertIn("mnt_namespace=4026533853\n", record)
        self.assertIn("uts_namespace=4026531838\n", record)
        self.assertIn("manager=verified\nenvironment=expected\nquery=sampled\n", record)
        self.assertIn("sample_count=8\nsampling=complete\n", record)
        self.assertIn("sample=1\n", record)
        self.assertIn("sample=8\n", record)
        self.assertIn("info_status=complete\n", record)
        self.assertIn("info_version=35240\n", record)
        self.assertIn("info_full_version=v4.2.0-rc3-85-gabc1234@BakaSU\n", record)
        self.assertIn("info_flags=0x1f\n", record)
        self.assertIn("info_uapi_version=6\n", record)
        self.assertIn("info_lkm=true\n", record)
        self.assertIn("info_bundled=false\n", record)
        self.assertIn("info_late_load=false\n", record)
        self.assertIn("info_runtime_mode=lkm\n", record)
        self.assertIn("info_pr_build=false\n", record)
        self.assertIn("version_status=complete\nversion=35240\n", record)
        self.assertIn("dmesg=available\n", record)
        self.assertIn("dmesg_init=present\n", record)
        self.assertIn("dmesg_init_version=35240\n", record)
        self.assertIn("dmesg_init_full_version=v4.2.0-rc3-85-gabc1234@BakaSU\n", record)
        self.assertIn("dmesg_init_abi=aarch64\n", record)
        self.assertIn("dmesg_init_work_mode=LKM\n", record)
        self.assertIn("dmesg_init_uapi_version=6\n", record)
        self.assertIn("dmesg_change_ksuver_values=32704,35240\n", record)
        self.assertIn("dmesg_change_ksuver_count=2\n", record)
        self.assertIn("config_ksu=y\n", record)
        self.assertIn("config_ksu_toolkit_support=n\n", record)
        self.assertIn("config_ksu_susfs=y\n", record)
        self.assertIn("config_ksu_multi_manager_support=y\n", record)
        self.assertIn("proc_modules=available\n", record)
        self.assertIn("proc_module_list=kernelsu:32768\n", record)
        self.assertIn("manager_version_file=present\n", record)
        self.assertIn("manager_version_owner=0:0\n", record)
        self.assertIn("manager_version_mode=600\n", record)
        self.assertIn("manager_version_value=35240\n", record)
        self.assertIn("module_names=alpha,ksu_toolkit\n", record)
        self.assertIn("module_count=2\n", record)
        self.assertIn("ksu_toolkit_dir=present\n", record)
        self.assertIn("ksu_toolkit_toolkit=present\n", record)
        self.assertIn("ksu_toolkit_system_bin=present\n", record)
        self.assertTrue(record.endswith("record=complete\n"))
        self.assertNotIn("unrelated secret", record)
        self.assertNotIn("features:", record)
        self.assertNotIn("forged", record)
        self.assertFalse(self.toolkit_marker.exists())
        self.assertFalse((self.case / "earkey-probe").exists())
        self.assertEqual(sorted(path.name for path in (self.case / ".kver-service").iterdir()), ["record"])
        self.assertEqual(stat.S_IMODE((self.case / ".kver-service").stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.case / ".kver-service/record").stat().st_mode), 0o600)
        self.assertEqual(len(self.manager_calls()), 16)
        self.assertEqual(self.manager_calls().count("debug info"), 8)
        self.assertEqual(self.manager_calls().count("debug version"), 8)
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertEqual(self.record(), record)
        self.assertEqual(len(self.manager_calls()), 16)

    def test_boot_completed_takes_one_snapshot(self):
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        record = self.record(stage="boot_completed")
        self.assertIn("hook=entered\nstage=boot_completed\n", record)
        self.assertIn("sample_count=1\nsampling=complete\n", record)
        self.assertNotIn("sample=2\n", record)
        self.assertEqual(len(self.manager_calls()), 2)

    def test_manager_mismatch_skips_queries_but_records_read_only_checks(self):
        self.assertEqual(self.run_hook(TEST_MANAGER_SHA="0" * 64).returncode, 0)
        record = self.record()
        self.assertIn("manager=hash_mismatch\n", record)
        self.assertIn("query=skipped_manager\n", record)
        self.assertNotIn("sample_count=", record)
        self.assertIn("dmesg=available\n", record)
        self.assertIn("config_ksu=y\n", record)
        self.assertEqual(self.manager_calls(), [])

    def test_environment_requires_userspace_and_uapi_but_not_kernel_version(self):
        self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE=None).returncode, 0)
        record = self.record()
        self.assertIn("env_kernel_version=unset\n", record)
        self.assertIn("environment=expected\nquery=sampled\n", record)
        self.assertIn("sample_count=8\n", record)
        self.assertEqual(len(self.manager_calls()), 16)

        for variable, value in (("KSU_VER_CODE", "1"), ("KSU_UAPI_VER", "1"), ("KSU_MODULE", "other")):
            with self.subTest(variable=variable):
                self.clear_attempt()
                (self.case / ".manager_calls").unlink(missing_ok=True)
                self.assertEqual(self.run_hook(**{variable: value}).returncode, 0)
                record = self.record()
                self.assertIn("environment=unexpected\nquery=skipped_environment\n", record)
                self.assertNotIn("sample_count=", record)
                self.assertEqual(self.manager_calls(), [])
                self.assertIn("dmesg=available\n", record)

    def test_malformed_manager_output_is_bounded(self):
        self.info_file.write_text(
            "version: 35240\n"
            "version: 35240\n"
            "full_version: bad value\n"
            "flags: 0x1f\n"
            "uapi_version: 6\n"
            "lkm: true\n"
            "bundled: false\n"
            "late_load: false\n"
            "runtime_mode: lkm\n"
            "pr_build: false\n"
            "info_status=forged\n"
        )
        self.version_file.write_text("Kernel Version: 42\nforged=value\n")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("info_version=duplicate\n", record)
        self.assertIn("info_full_version=invalid\n", record)
        self.assertIn("info_status=invalid\n", record)
        self.assertIn("version_status=invalid\n", record)
        self.assertNotIn("forged", record)
        self.assertNotIn("bad value", record)

    def test_query_timeout_is_classified(self):
        self.assertEqual(self.run_hook(TEST_INFO_TIMEOUT="1", TEST_VERSION_TIMEOUT="1").returncode, 0)
        record = self.record()
        self.assertIn("info_result=timeout\n", record)
        self.assertIn("version_result=timeout\n", record)
        self.assertNotIn("info_status=", record)
        self.assertNotIn("version_status=", record)
        self.assertEqual(self.manager_calls(), [])

    def test_dmesg_extracts_only_structured_values(self):
        self.dmesg_input.write_text(
            "[    0.100000] Initialized with driver version: notanumber, full_version: bad value, "
            "ABI: aarch64, Work mode: LKM, ksu_uapi_version: 6\n"
            "[   13.000000] handle_toolkit_cmd: ksu_change_ksuver to: -1\n"
            "[   13.100000] unrelated secret line\n"
        )
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("dmesg=available\n", record)
        self.assertIn("dmesg_init_count=1\n", record)
        self.assertIn("dmesg_init=invalid\n", record)
        self.assertIn("dmesg_change_ksuver_values=none\n", record)
        self.assertIn("dmesg_change_ksuver_invalid=1\n", record)
        self.assertNotIn("notanumber", record)
        self.assertNotIn("bad value", record)
        self.assertNotIn("unrelated secret", record)

    def test_dmesg_init_survives_unrelated_lines(self):
        lines = ["[    0.100000] KernelSU: Initialized with driver version: 35240, "
                 "full_version: v4.2.0-rc3-85-gabc1234@BakaSU, ABI: aarch64, "
                 "Work mode: LKM, ksu_uapi_version: 6"]
        for index in range(500):
            lines.append(f"[    1.{index:06d}] unrelated kernel line")
        lines.append("[   13.000000] handle_toolkit_cmd: ksu_change_ksuver to: 32704")
        self.dmesg_input.write_text("\n".join(lines) + "\n")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("dmesg_init=present\n", record)
        self.assertIn("dmesg_init_version=35240\n", record)
        self.assertIn("dmesg_change_ksuver_values=32704\n", record)
        self.assertNotIn("unrelated kernel line", record)

    def test_dmesg_change_values_are_ordered_and_bounded(self):
        lines = ["[   13.000000] Initialized with driver version: 35240, "
                 "full_version: v4.2.0-rc3-85-gabc1234@BakaSU, ABI: aarch64, "
                 "Work mode: LKM, ksu_uapi_version: 6"]
        for index in range(18):
            lines.append(f"[   14.{index:06d}] handle_toolkit_cmd: ksu_change_ksuver to: {40000 + index}")
        self.dmesg_input.write_text("\n".join(lines) + "\n")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        values = ",".join(str(40000 + index) for index in range(16))
        self.assertIn(f"dmesg_change_ksuver_values={values}\n", record)
        self.assertIn("dmesg_change_ksuver_count=18\n", record)
        self.assertIn("dmesg_change_ksuver_overflow=1\n", record)

    def test_config_checks_and_disabled_values(self):
        self.config_input.write_text(
            "# CONFIG_KSU is not set\n"
            "# CONFIG_KSU_TOOLKIT_SUPPORT is not set\n"
            "CONFIG_KSU_SUSFS=y\n"
            "CONFIG_KSU_MULTI_MANAGER_SUPPORT=n\n"
        )
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("config=available\n", record)
        self.assertIn("config_ksu=n\n", record)
        self.assertIn("config_ksu_toolkit_support=n\n", record)
        self.assertIn("config_ksu_susfs=y\n", record)
        self.assertIn("config_ksu_multi_manager_support=n\n", record)

    def test_config_absence_is_explicit(self):
        self.config_input.unlink()
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("config=unavailable\n", record)
        self.assertIn("config_ksu=unavailable\n", record)
        self.assertIn("config_ksu_toolkit_support=unavailable\n", record)

    def test_manager_version_metadata_and_validation(self):
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("manager_version_file=present\n", record)
        self.assertIn("manager_version_owner=0:0\n", record)
        self.assertIn("manager_version_mode=600\n", record)
        self.assertIn("manager_version_value=35240\n", record)

        self.clear_attempt()
        self.manager_version_path.write_text("12345678901\nforged=value\n")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("manager_version_value=invalid\n", record)
        self.assertNotIn("forged", record)

        self.clear_attempt()
        self.manager_version_path.unlink()
        self.manager_version_path.symlink_to("missing")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("manager_version_file=symlink\n", record)

    def test_module_presence_and_deadline(self):
        self.assertEqual(self.run_hook(TEST_SLEEP_JUMP="1").returncode, 0)
        record = self.record()
        self.assertIn("sampling=deadline\n", record)
        self.assertIn("sample_count=1\n", record)
        self.assertIn("module_names=alpha,ksu_toolkit\n", record)
        self.assertIn("ksu_toolkit_toolkit=present\n", record)
        self.assertFalse(self.toolkit_marker.exists())

        self.clear_attempt()
        shutil.rmtree(self.modules_root / "ksu_toolkit")
        self.assertEqual(self.run_hook().returncode, 0)
        record = self.record()
        self.assertIn("module_names=alpha\n", record)
        self.assertIn("ksu_toolkit_dir=absent\n", record)
        self.assertIn("ksu_toolkit_toolkit=absent\n", record)

    def test_disable_remove_and_existing_attempt_prevent_execution(self):
        for marker in ("disable", "remove", ".kver-service"):
            with self.subTest(marker=marker):
                path = self.case / marker
                path.symlink_to("missing")
                self.assertEqual(self.run_hook().returncode, 0)
                self.assertEqual(self.manager_calls(), [])
                path.unlink()

    def test_package_is_reproducible_and_contains_no_probe(self):
        archives = [self.root / "one.zip", self.root / "two.zip"]
        for archive in archives:
            subprocess.run(["python3", str(PACKAGER), str(archive)], check=True)
        self.assertEqual(archives[0].read_bytes(), archives[1].read_bytes())
        with zipfile.ZipFile(archives[0]) as archive:
            self.assertIsNone(archive.testzip())
            self.assertEqual(archive.namelist(), [
                "LICENSE", "boot-completed.sh", "customize.sh", "module.prop",
                "service.sh", "skip_mount",
            ])
            self.assertNotIn("earkey-probe", archive.namelist())
            self.assertNotIn("probe.sha256", archive.namelist())
            self.assertIn(b"id=earkey_control_kver", archive.read("module.prop"))
            self.assertNotIn(b"KSU_KERNEL_VER_CODE", archive.read("customize.sh"))
            self.assertEqual(archive.read("skip_mount"), b"")
            modes = {info.filename: (info.external_attr >> 16) & 0o777 for info in archive.infolist()}
            self.assertEqual(modes["service.sh"], 0o755)
            self.assertEqual(modes["boot-completed.sh"], 0o755)
            self.assertEqual(modes["customize.sh"], 0o644)
            self.assertEqual(modes["module.prop"], 0o644)

    def test_installer_accepts_kernel_version_mismatch(self):
        env = self.env | {
            "BOOTMODE": "true", "KSU": "true", "BAKASU": "true",
            "KSU_VER_CODE": "35257", "KSU_UAPI_VER": "6",
            "ARCH": "arm64", "API": "37",
            "MODPATH": "/data/adb/modules_update/earkey_control_kver",
            "KSU_KERNEL_VER_CODE": "32704",
        }
        command = 'abort() { printf "abort\\n"; exit 1; }; . ./customize.sh; printf "ok\\n"'
        result = subprocess.run(["dash", "-c", command], cwd=self.case, env=env,
                                capture_output=True, text=True, timeout=8)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "ok\n")
        for variable, value in (("KSU_VER_CODE", "1"), ("MODPATH", "/wrong"),
                                ("KSU_UAPI_VER", "1"), ("ARCH", "arm")):
            with self.subTest(variable=variable):
                bad_env = env | {variable: value}
                result = subprocess.run(["dash", "-c", command], cwd=self.case, env=bad_env,
                                        capture_output=True, text=True, timeout=8)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, "abort\n")


if __name__ == "__main__":
    unittest.main()
