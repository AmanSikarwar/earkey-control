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
FIXTURE = ROOT / "tests/fixtures/resukisu-service-preflight"
ELF = ROOT / "target/stage4/1506082/earkey-probe-arm64-api31"
PACKAGER = ROOT / "tools/package-service-probe.py"


class ServicePreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.case = self.root / "module"
        shutil.copytree(FIXTURE, self.case)
        self.manager = self.case / "manager"
        self.manager.write_text('''#!/bin/sh
[ "$*" = "debug info" ] || exit 98
printf "ran\\n" >> .manager_calls
printf "version: %s\\n" "${TEST_INFO_VERSION:-35201}"
printf "full_version: %s\\n" "${TEST_INFO_FULL:-v4.2.0-35201-backslashxx}"
printf "flags: %s\\n" "${TEST_INFO_FLAGS:-0x160}"
printf "uapi_version: %s\\n" "${TEST_INFO_UAPI:-6}"
printf "features: %s\\n" "${TEST_INFO_FEATURES:-0x7}"
printf "lkm: %s\\n" "${TEST_INFO_LKM:-false}"
printf "bundled: %s\\n" "${TEST_INFO_BUNDLED:-false}"
printf "late_load: %s\\n" "${TEST_INFO_LATE:-false}"
printf "runtime_mode: %s\\n" "${TEST_INFO_MODE:-built-in}"
printf "pr_build: %s\\n" "${TEST_INFO_PR:-false}"
''')
        self.manager.chmod(0o755)
        self.context_input = self.case / "context_input"
        self.context_input.write_text("u:r:test:s0")
        self.uptime_input = self.case / "uptime_input"
        self.uptime_input.write_text("123.45 678.9\n")
        self.kernel_version_input = self.case / "kernel_version_input"
        self.kernel_version_input.write_text(
            "Linux version 4.14.357-reforge-g9b21e3df6c0a (builder@example) "
            "(gcc version test) #2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026\n")
        self.modules_input = self.case / "modules_input"
        self.modules_input.write_text("")
        self.fd_root = self.root / "proc"
        (self.fd_root / "fd").mkdir(parents=True)
        (self.fd_root / "fdinfo").mkdir()
        self.add_fd("3", "anon_inode:[ksu_driver]", "flags:\t02000000\n")
        self.add_fd("4", "anon_inode:[ksu_driver_su]", "flags:\t00000002\n")
        self.add_fd("5", "/private/unrelated", "flags:\t00000002\n")
        hook = self.case / "service.sh"
        hook.write_text(hook.read_text().replace("/data/adb/ksud", str(self.manager))
                        .replace('"/proc/$$/attr/current"', shlex.quote(str(self.context_input)))
                        .replace("/proc/uptime", shlex.quote(str(self.uptime_input)))
                        .replace("/proc/version", shlex.quote(str(self.kernel_version_input)))
                        .replace("/proc/modules", shlex.quote(str(self.modules_input)))
                        .replace("/proc/$$/fdinfo", str(self.fd_root / "fdinfo"))
                        .replace("/proc/$$/fd", str(self.fd_root / "fd")))
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.mock("busybox", 'exec "$@"')
        boot_hook = self.case / "boot-completed.sh"
        boot_hook.write_text(boot_hook.read_text().replace("/data/adb/ksu/bin/busybox", str(self.bin / "busybox")))
        self.mock("id", '[ "$1" = -u ] && printf "%s\\n" "${TEST_UID:-0}"')
        real_stat = shlex.quote(shutil.which("stat"))
        self.mock("stat", f'''if [ "${{3:-}}" = "$TEST_MANAGER_PATH" ]; then
    printf "%s\\n" "${{TEST_MANAGER_STAT:-0:0:755}}"
else
    exec {real_stat} "$@"
fi''')
        self.mock(
            "timeout",
'''[ "$1" = -s ] && [ "$2" = KILL ] || exit 96
[ "$3" = 5 ] || [ "$3" = 1 ] || exit 96
shift 3
case "$1" in
    /system/bin/readlink)
        case "$2" in */fd/*) exec readlink "$2" ;; esac
        case "${2##*/}" in
            mnt) printf '%s\\n' "${TEST_MNT_NS:-mnt:[123]}" ;;
            uts) printf '%s\\n' "${TEST_UTS_NS:-uts:[456]}" ;;
            *) exit 96 ;;
        esac
        exit 0 ;;
    /system/bin/sha256sum)
        shift
        if [ "$1" = "$TEST_MANAGER_PATH" ]; then
            printf '%s  %s\\n' "${TEST_MANAGER_SHA:-fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50}" "$1"
            exit 0
        fi
        exec sha256sum "$@" ;;
    "$TEST_MANAGER_PATH") [ "${TEST_MANAGER_TIMEOUT:-0}" = 1 ] && exit 137 ;;
    ./earkey-probe) [ "${TEST_TIMEOUT:-0}" = 1 ] && exit 137 ;;
    *) exit 96 ;;
esac
exec "$@"''',
        )
        probe = self.case / "earkey-probe"
        probe.write_text('#!/bin/sh\n[ "$*" = self-check ] || exit 97\nprintf "ran\\n" >> .calls\nprintf "probe=self_check_ok\\n"\n')
        probe.chmod(0o500)
        (self.case / "probe.sha256").write_text(hashlib.sha256(probe.read_bytes()).hexdigest() + "\n")
        self.env = os.environ | {
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "KSU": "true", "BAKASU": "true", "KSU_MODULE": "earkey_control_preflight",
            "KSU_VER_CODE": "35257", "KSU_KERNEL_VER_CODE": "35201", "KSU_UAPI_VER": "6",
            "TEST_MANAGER_PATH": str(self.manager),
        }

    def mock(self, name, body):
        path = self.bin / name
        path.write_text(f"#!/bin/sh\n{body}\n")
        path.chmod(0o700)

    def add_fd(self, number, target, info):
        (self.fd_root / "fd" / number).symlink_to(target)
        if info is not None:
            (self.fd_root / "fdinfo" / number).write_text(info)

    def run_hook(self, *, stage="service", **changes):
        env = self.env | {key: value for key, value in changes.items() if value is not None}
        for key, value in changes.items():
            if value is None:
                env.pop(key, None)
        return subprocess.run(
            ["dash", str(self.case / ("boot-completed.sh" if stage == "boot_completed" else "service.sh"))],
            env=env, capture_output=True, text=True, timeout=8,
        )

    def result(self):
        return (self.case / ".preflight/result").read_text()

    def reset(self):
        shutil.rmtree(self.case / ".preflight", ignore_errors=True)
        for name in (".manager_calls", ".calls"):
            (self.case / name).unlink(missing_ok=True)

    def set_manager_output(self, output):
        self.manager.write_text(
            "#!/bin/sh\ncat <<'EOF'\n" + output + "EOF\nprintf 'ran\\n' >> .manager_calls\n")
        self.manager.chmod(0o755)

    def assert_no_probe(self):
        self.assertFalse((self.case / ".calls").exists())

    def test_success_is_private_and_runs_once(self):
        self.assertEqual(self.run_hook().returncode, 0)
        result = self.result()
        self.assertIn("hook=entered\nkernel_version_state=numeric\n"
                      "kernel_version_value=35201\n", result)
        self.assertIn("manager_identity=verified\n", result)
        self.assertIn("info_full_version=v4.2.0-35201-backslashxx\ninfo_flags=0x160\n"
                      "info_features=0x7\ninfo_uapi_version=6\n", result)
        self.assertIn("info_lkm=false\ninfo_bundled=false\ninfo_late_load=false\n", result)
        self.assertIn("info_runtime_mode=built-in\ninfo_pr_build=false\n"
                      "info_status=complete\ninfo_result=ok\n", result)
        self.assertIn("driver_identity=verified\nkernel_identity=verified\n"
                      "modules_identity=verified\n", result)
        self.assertIn("launcher=resukisu_service\n", result)
        self.assertIn("probe=self_check_ok\nservice_probe_exit=0\n", result)
        self.assertEqual(stat.S_IMODE((self.case / ".preflight").stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.case / ".preflight/result").stat().st_mode), 0o600)
        self.assertFalse((self.case / ".preflight/result.tmp").exists())
        context = (self.case / ".preflight/context").read_text()
        self.assertEqual(context, "hook_uptime_seconds=123.45\nselinux_context=u:r:test:s0\n"
                         "mnt_namespace=123\nuts_namespace=456\nfd_scan=complete\n"
                         "driver_cloexec_yes=1\ndriver_cloexec_no=0\ndriver_cloexec_unknown=0\n"
                         "driver_su_cloexec_yes=0\ndriver_su_cloexec_no=1\ndriver_su_cloexec_unknown=0\n")
        self.assertEqual(stat.S_IMODE((self.case / ".preflight/context").stat().st_mode), 0o600)
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\n")
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertEqual((self.case / ".calls").read_text(), "ran\n")
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\n")

    def test_each_environment_guard_records_failure_without_executing_probe(self):
        cases = [
            ("KSU", "false", "ksu"),
            ("BAKASU", "false", "bakasu"),
            ("KSU_MODULE", "other", "module"),
            ("KSU_VER_CODE", "1", "userspace_version"),
            ("KSU_UAPI_VER", "1", "uapi_version"),
            ("TEST_UID", "2000", "uid"),
        ]
        for variable, value, label in cases:
            with self.subTest(guard=label):
                self.reset()
                self.assertEqual(self.run_hook(**{variable: value}).returncode, 0)
                self.assertEqual(self.result(),
                                 f"hook=entered\nguard_failure={label}\nservice_probe_exit=1\n")
                self.assertFalse((self.case / ".manager_calls").exists())
                self.assert_no_probe()

    def test_mutable_kernel_version_is_observation_only(self):
        cases = [
            (None, "unset", None),
            ("", "empty", None),
            ("32704", "numeric", "32704"),
            ("35240", "numeric", "35240"),
            ("000035240", "numeric", "000035240"),
            ("12345678901", "invalid", None),
            ("42\nforged=value", "invalid", None),
            ("-1", "invalid", None),
            ("٤٢", "invalid", None),
        ]
        for value, state, number in cases:
            with self.subTest(value=value):
                self.reset()
                self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE=value).returncode, 0)
                result = self.result()
                self.assertIn(f"kernel_version_state={state}\n", result)
                if number is None:
                    self.assertNotIn("kernel_version_value=", result)
                else:
                    self.assertIn(f"kernel_version_value={number}\n", result)
                self.assertIn("manager_identity=verified\n", result)
                self.assertIn("driver_identity=verified\nkernel_identity=verified\n"
                              "modules_identity=verified\n", result)
                self.assertIn("service_probe_exit=0\n", result)
                self.assertTrue((self.case / ".calls").exists())

    def test_driver_version_is_observation_only(self):
        for value in ("32704", "35201", "35240", "000035201"):
            with self.subTest(value=value):
                self.reset()
                self.assertEqual(self.run_hook(TEST_INFO_VERSION=value).returncode, 0)
                result = self.result()
                self.assertIn(f"info_version={value}\n", result)
                self.assertIn("driver_identity=verified\nkernel_identity=verified\n"
                              "modules_identity=verified\n", result)
                self.assertIn("service_probe_exit=0\n", result)
                self.assertTrue((self.case / ".calls").exists())

    def test_composite_identity_rejects_untrusted_manager(self):
        cases = [
            {"TEST_MANAGER_SHA": "0" * 64},
            {"TEST_MANAGER_STAT": "501:0:755"},
        ]
        for changes in cases:
            with self.subTest(changes=changes):
                self.reset()
                self.assertEqual(self.run_hook(**changes).returncode, 0)
                result = self.result()
                self.assertIn("manager_identity=unverified\n", result)
                self.assertIn("info_result=unavailable\n", result)
                self.assertIn("driver_identity=not_checked\n", result)
                self.assertIn("guard_failure=manager_identity\n"
                              "service_probe_exit=1\n", result)
                self.assertFalse((self.case / ".manager_calls").exists())
                self.assert_no_probe()

    def test_composite_identity_requires_expected_driver(self):
        cases = [
            ({"TEST_INFO_FULL": "unknown"}, "mismatch", "full_version"),
            ({"TEST_INFO_FULL": "v4.2.0-35200-other"}, "mismatch", "full_version"),
            ({"TEST_INFO_FLAGS": "0x0"}, "mismatch", "flags"),
            ({"TEST_INFO_FLAGS": "0x120"}, "mismatch", "flags"),
            ({"TEST_INFO_FEATURES": "0x3"}, "mismatch", "features"),
            ({"TEST_INFO_UAPI": "5"}, "mismatch", "uapi_version"),
            ({"TEST_INFO_LKM": "true"}, "mismatch", "lkm"),
            ({"TEST_INFO_BUNDLED": "true"}, "mismatch", "bundled"),
            ({"TEST_INFO_LATE": "true"}, "mismatch", "late_load"),
            ({"TEST_INFO_MODE": "lkm"}, "mismatch", "runtime_mode"),
            ({"TEST_INFO_PR": "true"}, "mismatch", "pr_build"),
            ({"TEST_INFO_FLAGS": "bad"}, "invalid", None),
            ({"TEST_INFO_FEATURES": "bad"}, "invalid", None),
            ({"TEST_INFO_UAPI": "12345678901"}, "invalid", None),
        ]
        for changes, identity, reason in cases:
            with self.subTest(changes=changes):
                self.reset()
                self.assertEqual(self.run_hook(**changes).returncode, 0)
                result = self.result()
                self.assertIn("manager_identity=verified\n", result)
                self.assertIn("guard_failure=driver_identity\n", result)
                self.assertIn(f"driver_identity={identity}\n", result)
                if reason is None:
                    self.assertNotIn("driver_reason=", result)
                else:
                    self.assertIn(f"driver_reason={reason}\n", result)
                self.assertIn("service_probe_exit=1\n", result)
                self.assertTrue((self.case / ".manager_calls").exists())
                self.assert_no_probe()

    def test_composite_identity_rejects_missing_duplicate_or_extra_info(self):
        prefix = ("version: 35201\nfull_version: v4.2.0-35201-backslashxx\n"
                  "flags: 0x160\nfeatures: 0x7\nuapi_version: 6\nlkm: false\n"
                  "bundled: false\nlate_load: false\nruntime_mode: built-in\n"
                  "pr_build: false\n")
        cases = [
            ("missing", prefix.replace("flags: 0x160\n", "")),
            ("duplicate", prefix.replace("flags: 0x160\n", "flags: 0x160\nflags: 0x160\n")),
        ]
        for label, output in cases:
            with self.subTest(label=label):
                self.reset()
                self.set_manager_output(output)
                self.assertEqual(self.run_hook().returncode, 0)
                result = self.result()
                self.assertIn(f"info_flags={label}\n", result)
                self.assertIn("info_status=invalid\n", result)
                self.assertIn("guard_failure=driver_identity\n"
                              "service_probe_exit=1\n", result)
                self.assert_no_probe()

        self.reset()
        self.set_manager_output(prefix.replace(
            "version: 35201\n", "version: 35201\nforged=value\n"))
        self.assertEqual(self.run_hook().returncode, 0)
        result = self.result()
        self.assertIn("info_version=35201\n", result)
        self.assertIn("info_status=complete\n", result)
        self.assertIn("service_probe_exit=0\n", result)
        self.assertNotIn("forged", result)

    def test_composite_identity_requires_measured_kernel(self):
        cases = [
            ("mismatch", "Linux version 4.14.357-openela-VantomKernel #1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026\n"),
            ("mismatch", "Linux version 4.14.357-reforge-g9b21e3df6c0a #1 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026\n"),
            ("mismatch", "Linux version 4.14.357-reforge-g9b21e3df6c0a #2 SMP PREEMPT Tue Nov 10 19:36:49 WIB 2026\n"),
        ]
        for identity, content in cases:
            with self.subTest(identity=identity, content=content):
                self.reset()
                self.kernel_version_input.write_text(content)
                self.assertEqual(self.run_hook().returncode, 0)
                result = self.result()
                self.assertIn(f"kernel_identity={identity}\n", result)
                self.assertIn("guard_failure=kernel_identity\n"
                              "service_probe_exit=1\n", result)
                self.assert_no_probe()

        self.reset()
        self.kernel_version_input.unlink()
        self.assertEqual(self.run_hook().returncode, 0)
        result = self.result()
        self.assertIn("kernel_identity=unavailable\n", result)
        self.assertIn("guard_failure=kernel_identity\nservice_probe_exit=1\n", result)
        self.assert_no_probe()

    def test_composite_identity_rejects_loaded_ksu_module(self):
        cases = [
            ("kernelsu 12345 0 - Live 0x0000000000000000\n", "conflict"),
            ("ksu 12345 0 - Live 0x0000000000000000\n", "conflict"),
            ("susfs 12345 0 - Live 0x0000000000000000\n", "conflict"),
        ]
        for content, identity in cases:
            with self.subTest(content=content):
                self.reset()
                self.modules_input.write_text(content)
                self.assertEqual(self.run_hook().returncode, 0)
                result = self.result()
                self.assertIn(f"modules_identity={identity}\n", result)
                self.assertIn("guard_failure=module_identity\n"
                              "service_probe_exit=1\n", result)
                self.assert_no_probe()

        self.reset()
        self.modules_input.unlink()
        self.assertEqual(self.run_hook().returncode, 0)
        result = self.result()
        self.assertIn("modules_identity=unavailable\n", result)
        self.assertIn("guard_failure=module_identity\nservice_probe_exit=1\n", result)
        self.assert_no_probe()

    def test_symlink_manager_is_not_queried(self):
        self.manager.rename(self.case / "real_manager")
        self.manager.symlink_to("real_manager")
        self.assertEqual(self.run_hook().returncode, 0)
        result = self.result()
        self.assertIn("manager_identity=unverified\n", result)
        self.assertIn("guard_failure=manager_identity\nservice_probe_exit=1\n", result)
        self.assertFalse((self.case / ".manager_calls").exists())
        self.assert_no_probe()

    def test_boot_completed_compares_without_running_probe(self):
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        result = (self.case / ".boot-preflight/result").read_text()
        self.assertIn("kernel_version_state=numeric\nkernel_version_value=35201\n", result)
        self.assertIn("manager_identity=verified\n", result)
        self.assertIn("info_runtime_mode=built-in\ninfo_pr_build=false\n"
                      "info_status=complete\ninfo_result=ok\n", result)
        self.assertIn("driver_identity=verified\nkernel_identity=verified\n"
                      "modules_identity=verified\n", result)
        self.assertTrue(result.endswith("guard_exit=0\n"))
        self.assert_no_probe()
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\n")
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\n")
        self.assertEqual(stat.S_IMODE((self.case / ".boot-preflight/context").stat().st_mode), 0o600)

    def test_boot_completed_failure_and_untrusted_namespace(self):
        self.assertEqual(self.run_hook(stage="boot_completed", TEST_INFO_FLAGS="0x0",
                                      TEST_MNT_NS="mnt:[42]\nforged=1").returncode, 0)
        result = (self.case / ".boot-preflight/result").read_text()
        self.assertIn("guard_failure=driver_identity\n", result)
        self.assertTrue(result.endswith("guard_exit=1\n"))
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertIn("mnt_namespace=unavailable\n", context)
        self.assertNotIn("forged", context)
        self.assert_no_probe()

    def test_context_rejects_untrusted_label(self):
        self.context_input.write_text("u:r:bad label:s0")
        self.uptime_input.write_text("12.3;forged 42\n")
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertTrue(context.startswith("hook_uptime_seconds=unavailable\nselinux_context=invalid\n"))
        self.assertNotIn("bad label", context)
        self.assertNotIn("forged", context)

    def test_descriptor_scan_limits_and_unknown_flags(self):
        self.add_fd("6", "anon_inode:[ksu_driver]", "flags:\tbad\n")
        self.add_fd("7", "anon_inode:[ksu_driver_su]", None)
        for number in range(8, 21):
            self.add_fd(str(number), "/private/unrelated", "flags:\t2\n")
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertIn("fd_scan=incomplete\n", context)
        self.assertNotIn("/private/unrelated", context)

    def test_descriptor_scan_unknown_flags(self):
        self.add_fd("6", "anon_inode:[ksu_driver]", "flags:\tbad\n")
        self.add_fd("7", "anon_inode:[ksu_driver_su]", None)
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertIn("driver_cloexec_unknown=1\n", context)
        self.assertIn("driver_su_cloexec_unknown=1\n", context)

    def test_boot_completed_honors_disable_and_existing_attempt(self):
        for marker in ("disable", "remove", ".boot-preflight"):
            with self.subTest(marker=marker):
                path = self.case / marker
                path.symlink_to("missing")
                self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
                self.assertFalse((self.case / ".manager_calls").exists())
                path.unlink()

    def test_disable_remove_and_existing_attempt_prevent_execution(self):
        for marker in ("disable", "remove", ".preflight"):
            with self.subTest(marker=marker):
                path = self.case / marker
                path.symlink_to("missing")
                self.assertEqual(self.run_hook().returncode, 0)
                self.assertFalse((self.case / ".calls").exists())
                path.unlink()

    def test_bad_digest_and_timeout_are_recorded(self):
        (self.case / "probe.sha256").write_text("0" * 64 + "\n")
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertIn("preflight=digest_mismatch\nservice_probe_exit=1\n", self.result())
        self.assertFalse((self.case / ".calls").exists())
        shutil.rmtree(self.case / ".preflight")
        probe = self.case / "earkey-probe"
        (self.case / "probe.sha256").write_text(hashlib.sha256(probe.read_bytes()).hexdigest() + "\n")
        self.assertEqual(self.run_hook(TEST_TIMEOUT="1").returncode, 0)
        self.assertTrue(self.result().endswith("service_probe_exit=137\n"))
        self.assertFalse((self.case / ".calls").exists())

    def test_symlink_probe_is_rejected(self):
        (self.case / "earkey-probe").rename(self.case / "payload")
        (self.case / "earkey-probe").symlink_to("payload")
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertIn("preflight=digest_mismatch", self.result())
        self.assertFalse((self.case / ".calls").exists())

    def test_installer_accepts_mutable_kernel_version(self):
        command = ('abort() { printf "abort\\n"; exit 1; }; '
                   'set_perm() { :; }; . ./customize.sh; printf "ok\\n"')
        env = self.env | {
            "BOOTMODE": "true", "KSU": "true", "BAKASU": "true",
            "KSU_VER_CODE": "35257", "KSU_UAPI_VER": "6",
            "ARCH": "arm64", "API": "37",
            "MODPATH": "/data/adb/modules_update/earkey_control_preflight",
        }
        for value in ("32704", "35240", "", None):
            with self.subTest(kernel_version=value):
                run_env = env | ({"KSU_KERNEL_VER_CODE": value} if value is not None else {})
                if value is None:
                    run_env.pop("KSU_KERNEL_VER_CODE", None)
                result = subprocess.run(["dash", "-c", command], cwd=self.case, env=run_env,
                                        capture_output=True, text=True, timeout=8)
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "ok\n")

        self.assertNotIn("KSU_KERNEL_VER_CODE", (FIXTURE / "customize.sh").read_text())
        for variable, value in (("BOOTMODE", "false"), ("KSU", "false"), ("BAKASU", "false"),
                                ("KSU_VER_CODE", "1"), ("KSU_UAPI_VER", "1"),
                                ("ARCH", "arm"), ("API", "31"), ("MODPATH", "/wrong")):
            with self.subTest(variable=variable):
                result = subprocess.run(["dash", "-c", command], cwd=self.case,
                                        env=env | {variable: value},
                                        capture_output=True, text=True, timeout=8)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, "abort\n")

    def test_package_is_reproducible_and_contains_the_pinned_probe(self):
        if not ELF.exists():
            self.skipTest("the locally built reviewed ELF is unavailable")
        archives = [self.root / "one.zip", self.root / "two.zip"]
        for archive in archives:
            subprocess.run(["python3", str(PACKAGER), str(ELF), str(archive)], check=True)
        self.assertEqual(archives[0].read_bytes(), archives[1].read_bytes())
        with zipfile.ZipFile(archives[0]) as archive:
            self.assertIsNone(archive.testzip())
            self.assertEqual(archive.namelist(), [
                "LICENSE", "boot-completed.sh", "customize.sh", "earkey-probe", "module.prop",
                "probe.sha256", "service.sh", "skip_mount",
            ])
            self.assertEqual(hashlib.sha256(archive.read("earkey-probe")).hexdigest(),
                             (FIXTURE / "probe.sha256").read_text().strip())
            self.assertIn(b"version=0.0.8-preflight\nversionCode=8\n", archive.read("module.prop"))
            self.assertNotIn(b"KSU_KERNEL_VER_CODE", archive.read("customize.sh"))
            self.assertEqual(archive.read("skip_mount"), b"")
            modes = {info.filename: (info.external_attr >> 16) & 0o777
                     for info in archive.infolist()}
            self.assertEqual(modes["service.sh"], 0o755)
            self.assertEqual(modes["boot-completed.sh"], 0o755)
            self.assertEqual(modes["customize.sh"], 0o644)
            self.assertEqual(modes["module.prop"], 0o644)
            self.assertEqual(modes["earkey-probe"], 0o500)


if __name__ == "__main__":
    unittest.main()
