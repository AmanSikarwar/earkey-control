#!/usr/bin/env python3
"""Package the installable earkey_control module from the reviewed worker ELF."""

import argparse
import hashlib
from pathlib import Path
import stat
import subprocess
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "module"
REGISTRY = Path.home() / ".cargo" / "registry" / "src"
LICENSE_PREFIXES = ("license", "copying", "unlicense", "notice")


def runtime_crates():
    """Registry crates linked into the Android worker, from the locked dependency tree."""
    output = subprocess.run(
        ["cargo", "tree", "--locked", "--offline", "-p", "earkey-worker",
         "--target", "aarch64-linux-android", "-e", "normal", "--prefix", "none",
         "--format", "{p}"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    crates = set()
    for line in output.splitlines():
        parts = line.replace(" (*)", "").split()
        if len(parts) == 2 and parts[1].startswith("v"):
            crates.add((parts[0], parts[1][1:]))
    return sorted(crates)


def license_files(name, version):
    sources = sorted(REGISTRY.glob(f"*/{name}-{version}"))
    if not sources:
        raise SystemExit(f"missing registry source for {name} {version}")
    files = sorted(p for p in sources[0].iterdir()
                   if p.is_file() and p.name.lower().startswith(LICENSE_PREFIXES))
    if not files:
        raise SystemExit(f"no license file in {name} {version}")
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("worker", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    expected = (MODULE / "earkey-worker.sha256").read_text(encoding="ascii").strip()
    worker = args.worker.read_bytes()
    if hashlib.sha256(worker).hexdigest() != expected:
        parser.error("worker digest differs from the reviewed artifact")

    files = {
        "module.prop": ((MODULE / "module.prop").read_bytes(), 0o644),
        "customize.sh": ((MODULE / "customize.sh").read_bytes(), 0o644),
        "service.sh": ((MODULE / "service.sh").read_bytes(), 0o755),
        "uninstall.sh": ((MODULE / "uninstall.sh").read_bytes(), 0o755),
        "skip_mount": (b"", 0o644),
        "bin/earkey-worker": (worker, 0o755),
        "bin/earkey-worker.sha256": (f"{expected}\n".encode(), 0o644),
        "LICENSE": ((ROOT / "LICENSE").read_bytes(), 0o644),
        "THIRD_PARTY_NOTICES.md": ((ROOT / "THIRD_PARTY_NOTICES.md").read_bytes(), 0o644),
    }
    for name, version in runtime_crates():
        for path in license_files(name, version):
            files[f"licenses/{name}-{version}/{path.name}"] = (path.read_bytes(), 0o644)

    with ZipFile(args.output, "x", compression=ZIP_DEFLATED, compresslevel=9) as archive:
        for name, (data, mode) in sorted(files.items()):
            member = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            member.create_system = 3
            member.external_attr = (stat.S_IFREG | mode) << 16
            member.compress_type = ZIP_DEFLATED
            archive.writestr(member, data)


if __name__ == "__main__":
    main()
