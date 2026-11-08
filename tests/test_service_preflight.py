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
        self.manager.write_text('#!/bin/sh\n[ "$*" = "debug version" ] || exit 98\nprintf "ran\\n" >> .manager_calls\nprintf "%s\\n" "${TEST_MANAGER_OUTPUT:-Kernel Version: 35240}"\n')
        self.manager.chmod(0o755)
        self.context_input = self.case / "context_input"
        self.context_input.write_text("u:r:test:s0")
        self.uptime_input = self.case / "uptime_input"
        self.uptime_input.write_text("123.45 678.9\n")
        hook = self.case / "service.sh"
        hook.write_text(hook.read_text().replace("/data/adb/ksud", str(self.manager))
                        .replace('"/proc/$$/attr/current"', shlex.quote(str(self.context_input)))
                        .replace("/proc/uptime", shlex.quote(str(self.uptime_input))))
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
            '''[ "$1" = -s ] && [ "$2" = KILL ] && [ "$3" = 5 ] || exit 96
shift 3
case "$1" in
    /system/bin/readlink)
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
            "KSU_VER_CODE": "35257", "KSU_KERNEL_VER_CODE": "35240", "KSU_UAPI_VER": "6",
            "TEST_MANAGER_PATH": str(self.manager),
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
            env=env, capture_output=True, text=True, timeout=8,
        )

    def result(self):
        return (self.case / ".preflight/result").read_text()

    def test_success_is_private_and_runs_once(self):
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertIn("hook=entered\nlauncher=resukisu_service\n", self.result())
        self.assertIn("probe=self_check_ok\nservice_probe_exit=0\n", self.result())
        self.assertEqual(stat.S_IMODE((self.case / ".preflight").stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.case / ".preflight/result").stat().st_mode), 0o600)
        self.assertFalse((self.case / ".preflight/result.tmp").exists())
        context = (self.case / ".preflight/context").read_text()
        self.assertEqual(context, "hook_uptime_seconds=123.45\nselinux_context=u:r:test:s0\n"
                         "mnt_namespace=123\nuts_namespace=456\n")
        self.assertEqual(stat.S_IMODE((self.case / ".preflight/context").stat().st_mode), 0o600)
        self.assertFalse((self.case / ".manager_calls").exists())
        self.assertEqual(self.run_hook().returncode, 0)
        self.assertEqual((self.case / ".calls").read_text(), "ran\n")

    def test_each_guard_records_failure_without_executing_probe(self):
        cases = [
            ("KSU", "false", "ksu"),
            ("BAKASU", "false", "bakasu"),
            ("KSU_MODULE", "other", "module"),
            ("KSU_VER_CODE", "1", "userspace_version"),
            ("KSU_KERNEL_VER_CODE", "1", "kernel_version"),
            ("KSU_UAPI_VER", "1", "uapi_version"),
            ("TEST_UID", "2000", "uid"),
        ]
        for variable, value, label in cases:
            with self.subTest(guard=label):
                shutil.rmtree(self.case / ".preflight", ignore_errors=True)
                (self.case / ".manager_calls").unlink(missing_ok=True)
                self.assertEqual(self.run_hook(**{variable: value}).returncode, 0)
                detail = "kernel_version_state=numeric\nkernel_version_value=1\n" if label == "kernel_version" else ""
                if label == "kernel_version":
                    detail += "fresh_query=numeric\nfresh_query_value=35240\n"
                self.assertEqual(self.result(), f"hook=entered\nguard_failure={label}\n{detail}service_probe_exit=1\n")
                self.assertFalse((self.case / ".calls").exists())
                self.assertEqual((self.case / ".manager_calls").exists(), label == "kernel_version")

    def test_kernel_version_failure_reports_only_bounded_numeric_values(self):
        cases = [
            (None, "unset", None),
            ("", "empty", None),
            ("000035240", "numeric", "000035240"),
            ("1234567890", "numeric", "1234567890"),
            ("12345678901", "invalid", None),
            ("42\nforged=value", "invalid", None),
            ("-1", "invalid", None),
            ("٤٢", "invalid", None),
        ]
        for value, state, number in cases:
            with self.subTest(value=value):
                shutil.rmtree(self.case / ".preflight", ignore_errors=True)
                self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE=value).returncode, 0)
                detail = f"kernel_version_value={number}\n" if number is not None else ""
                self.assertEqual(self.result(),
                    "hook=entered\nguard_failure=kernel_version\n"
                    f"kernel_version_state={state}\n{detail}"
                    "fresh_query=numeric\nfresh_query_value=35240\nservice_probe_exit=1\n")
                self.assertFalse((self.case / ".calls").exists())

    def test_fresh_query_requires_valid_manager_and_rejects_untrusted_output(self):
        cases = [
            ({"TEST_MANAGER_SHA": "0" * 64}, "unavailable", False),
            ({"TEST_MANAGER_STAT": "501:0:755"}, "unavailable", False),
            ({"KSU_UAPI_VER": "0"}, "unavailable", False),
            ({"TEST_UID": "2000"}, "unavailable", False),
            ({"TEST_MANAGER_TIMEOUT": "1"}, "failed", False),
            ({"TEST_MANAGER_OUTPUT": "Kernel Version: 12345678901"}, "invalid", True),
            ({"TEST_MANAGER_OUTPUT": "Kernel Version: 3\nforged=value"}, "invalid", True),
            ({"TEST_MANAGER_OUTPUT": "unexpected"}, "invalid", True),
        ]
        for changes, state, called in cases:
            with self.subTest(changes=changes):
                shutil.rmtree(self.case / ".preflight", ignore_errors=True)
                (self.case / ".manager_calls").unlink(missing_ok=True)
                self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE="32704", **changes).returncode, 0)
                self.assertEqual(self.result(), "hook=entered\nguard_failure=kernel_version\n"
                    "kernel_version_state=numeric\nkernel_version_value=32704\n"
                    f"fresh_query={state}\nservice_probe_exit=1\n")
                self.assertEqual((self.case / ".manager_calls").exists(), called)
                self.assertFalse((self.case / ".calls").exists())

    def test_symlink_manager_is_not_queried(self):
        self.manager.rename(self.case / "real_manager")
        self.manager.symlink_to("real_manager")
        self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE="32704").returncode, 0)
        self.assertIn("fresh_query=unavailable\n", self.result())
        self.assertFalse((self.case / ".manager_calls").exists())
        self.assertFalse((self.case / ".calls").exists())

    def test_boot_completed_compares_without_running_probe(self):
        self.assertEqual(self.run_hook(KSU_KERNEL_VER_CODE="32704", TEST_MANAGER_OUTPUT="Kernel Version: 32704").returncode, 0)
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        result = (self.case / ".boot-preflight/result").read_text()
        self.assertEqual(result, "hook=entered\nkernel_version_state=numeric\n"
                         "kernel_version_value=35240\nfresh_query=numeric\n"
                         "fresh_query_value=35240\nguard_exit=0\n")
        self.assertFalse((self.case / ".calls").exists())
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\nran\n")
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        self.assertEqual((self.case / ".manager_calls").read_text(), "ran\nran\n")
        self.assertEqual(stat.S_IMODE((self.case / ".boot-preflight/context").stat().st_mode), 0o600)

    def test_boot_completed_failure_and_untrusted_namespace(self):
        self.assertEqual(self.run_hook(stage="boot_completed", KSU_KERNEL_VER_CODE="32704",
                                      TEST_MNT_NS="mnt:[42]\nforged=1").returncode, 0)
        result = (self.case / ".boot-preflight/result").read_text()
        self.assertIn("guard_failure=kernel_version\n", result)
        self.assertTrue(result.endswith("guard_exit=1\n"))
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertIn("mnt_namespace=unavailable\n", context)
        self.assertNotIn("forged", context)
        self.assertFalse((self.case / ".calls").exists())

    def test_context_rejects_untrusted_label(self):
        self.context_input.write_text("u:r:bad label:s0")
        self.uptime_input.write_text("12.3;forged 42\n")
        self.assertEqual(self.run_hook(stage="boot_completed").returncode, 0)
        context = (self.case / ".boot-preflight/context").read_text()
        self.assertTrue(context.startswith("hook_uptime_seconds=unavailable\nselinux_context=invalid\n"))
        self.assertNotIn("bad label", context)
        self.assertNotIn("forged", context)

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
            self.assertEqual(archive.read("skip_mount"), b"")


if __name__ == "__main__":
    unittest.main()
