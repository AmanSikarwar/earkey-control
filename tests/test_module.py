import hashlib
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "module"
PACKAGER = ROOT / "tools/package-module.py"
ELF = ROOT / "target/stage6/cb437ed/earkey-worker-arm64-api31"
STORAGE = "/data/adb/earkey_control"


class ModuleTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.storage = self.root / "storage"
        self.bin = self.root / "bin-mocks"
        self.bin.mkdir()

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def script(self, name, directory):
        """Copies a module script with only the storage path redirected."""
        text = (MODULE / name).read_text().replace(STORAGE, str(self.storage))
        path = directory / name
        path.write_text(text)
        path.chmod(0o755)
        return path

    def install(self, **overrides):
        modpath = self.root / "modpath"
        (modpath / "bin").mkdir(parents=True, exist_ok=True)
        script = self.script("customize.sh", modpath)
        chown = self.bin / "chown"
        chown.write_text("#!/bin/sh\n[ \"$1\" = 0:0 ]\n")
        chown.chmod(0o755)
        env = {"PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}", "BOOTMODE": "true", "KSU": "true",
               "ARCH": "arm64", "API": "37", "MODPATH": str(modpath)} | overrides
        env = {k: v for k, v in env.items() if v is not None}
        command = ('abort() { printf "abort:%s\\n" "$1"; exit 1; }; '
                   'set_perm() { :; }; ui_print() { :; }; '
                   'set_perm_recursive() { abort "! Installer must preserve manager permissions"; }; '
                   f'. {script}; printf "ok\\n"')
        return subprocess.run(["dash", "-c", command], env=env, capture_output=True,
                              text=True, timeout=8)

    def test_installer_requires_kernelsu_boot_mode_arm64_and_api_31(self):
        for change in ({"BOOTMODE": "false"}, {"KSU": None}, {"KSU": "false"},
                       {"ARCH": "arm"}, {"API": "30"}, {"API": "thirty"}, {"API": None}):
            with self.subTest(change=change):
                result = self.install(**change)
                self.assertEqual(result.returncode, 1, result.stdout)
                self.assertTrue(result.stdout.startswith("abort:"), result.stdout)
                self.assertFalse(self.storage.exists())

    def test_installer_creates_private_storage_and_keeps_existing_settings(self):
        result = self.install()
        self.assertEqual(result.stdout, "ok\n")
        self.assertEqual(stat.S_IMODE(self.storage.stat().st_mode), 0o700)
        saved = {"policy.json": '{"recovery":{"paused":true}}', "enrollments.json": "{}", "stay-inactive": ""}
        for name, contents in saved.items():
            (self.storage / name).write_text(contents)
        self.storage.chmod(0o755)
        self.assertEqual(self.install().stdout, "ok\n")
        for name, contents in saved.items():
            self.assertEqual((self.storage / name).read_text(), contents)
        self.assertEqual(stat.S_IMODE(self.storage.stat().st_mode), 0o700)

    def test_installer_refuses_a_symlinked_storage(self):
        target = self.root / "elsewhere"
        target.mkdir()
        self.storage.symlink_to(target)
        result = self.install()
        self.assertEqual(result.returncode, 1)
        self.assertIn("symlink", result.stdout)

    def service(self, digest=None, storage=True, link=False):
        moddir = self.root / "moddir"
        (moddir / "bin").mkdir(parents=True, exist_ok=True)
        worker = moddir / "bin/earkey-worker"
        payload = self.root / "payload"
        payload.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "${0%/*}/../calls"\n')
        payload.chmod(0o755)
        if link:
            worker.symlink_to(payload)
        else:
            shutil.copy(payload, worker)
        sha = digest or hashlib.sha256(worker.read_bytes()).hexdigest()
        (moddir / "bin/earkey-worker.sha256").write_text(sha + "\n")
        if storage:
            self.storage.mkdir(mode=0o700, exist_ok=True)
        timeout = self.bin / "timeout"
        timeout.write_text('#!/bin/sh\nshift 3\n[ "$1" = /system/bin/sha256sum ] && { shift; exec sha256sum "$@"; }\nexit 96\n')
        timeout.chmod(0o755)
        script = self.script("service.sh", moddir)
        env = {"PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}"}
        subprocess.run(["dash", str(script)], env=env, timeout=8, check=True)
        deadline = 50
        calls = moddir / "calls"
        while deadline and not calls.exists():
            subprocess.run(["sleep", "0.02"])
            deadline -= 1
        return calls.read_text() if calls.exists() else None

    def test_service_starts_only_the_reviewed_daemon(self):
        self.assertEqual(self.service(), "daemon\n")

    def test_service_refuses_a_changed_binary_missing_storage_or_symlink(self):
        self.assertIsNone(self.service(digest="0" * 64))
        shutil.rmtree(self.root / "moddir")
        self.storage.rmdir()
        self.assertIsNone(self.service(storage=False))
        shutil.rmtree(self.root / "moddir")
        self.assertIsNone(self.service(link=True))

    def uninstall(self):
        moddir = self.root / "moddir"
        (moddir / "bin").mkdir(parents=True, exist_ok=True)
        script = self.script("uninstall.sh", moddir)
        subprocess.run(["dash", str(script)], timeout=8, check=True)

    def test_uninstall_removes_only_the_private_storage(self):
        self.storage.mkdir(mode=0o700)
        (self.storage / "policy.json").write_text("{}")
        (self.storage / "enrollments.json").write_text("{}")
        self.uninstall()
        self.assertFalse(self.storage.exists())

    def test_uninstall_does_not_follow_a_symlinked_storage(self):
        target = self.root / "elsewhere"
        target.mkdir()
        (target / "keep").write_text("keep")
        self.storage.symlink_to(target)
        self.uninstall()
        self.assertEqual((target / "keep").read_text(), "keep")

    def test_package_is_reproducible_and_complete(self):
        if not ELF.exists():
            self.skipTest("the locally built reviewed worker ELF is unavailable")
        archives = [self.root / "one.zip", self.root / "two.zip"]
        for archive in archives:
            subprocess.run(["python3", str(PACKAGER), str(ELF), str(archive)], check=True)
        self.assertEqual(archives[0].read_bytes(), archives[1].read_bytes())
        with zipfile.ZipFile(archives[0]) as archive:
            self.assertIsNone(archive.testzip())
            names = archive.namelist()
            for required in ("module.prop", "customize.sh", "service.sh", "uninstall.sh",
                             "skip_mount", "bin/earkey-worker", "bin/earkey-worker.sha256",
                             "LICENSE", "THIRD_PARTY_NOTICES.md", "webroot/index.html",
                             "webroot/style.css", "webroot/main.js", "webroot/status.js"):
                self.assertIn(required, names)
            self.assertFalse(any(n.startswith("webroot/") and n.endswith(".ts") for n in names))
            page = archive.read("webroot/index.html").decode()
            self.assertIn("script-src 'self'", page)
            self.assertNotIn("http", page.replace('http-equiv', ''))
            self.assertFalse(any(n.startswith(("module/", "earkey")) for n in names))
            for crate in ("sha2-0.11.1", "libc-0.2.191", "serde_json-1.0.154", "typenum-1.20.1"):
                self.assertTrue(any(n.startswith(f"licenses/{crate}/") for n in names), crate)
            prop = archive.read("module.prop").decode()
            self.assertIn("id=earkey_control\n", prop)
            self.assertIn("description=Configurable blocking of wired earphone button actions.\n", prop)
            self.assertEqual(hashlib.sha256(archive.read("bin/earkey-worker")).hexdigest(),
                             (MODULE / "earkey-worker.sha256").read_text().strip())
            modes = {i.filename: (i.external_attr >> 16) & 0o777 for i in archive.infolist()}
            self.assertEqual(modes["service.sh"], 0o755)
            self.assertEqual(modes["uninstall.sh"], 0o755)
            self.assertEqual(modes["customize.sh"], 0o644)
            for name in ("module.prop", "customize.sh", "service.sh", "uninstall.sh"):
                self.assertNotIn(b"\r\n", archive.read(name))


if __name__ == "__main__":
    unittest.main()
