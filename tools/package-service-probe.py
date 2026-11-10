#!/usr/bin/env python3
"""Package the exact Stage 4 worker ELF for the reference manager's native installer."""

import argparse
import hashlib
from pathlib import Path
import stat
from zipfile import ZIP_STORED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "resukisu-service-preflight"
WORKER_SIZE = 572_608


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("worker", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    expected = (FIXTURE / "worker.sha256").read_text(encoding="ascii").strip()
    if args.worker.stat().st_size != WORKER_SIZE:
        parser.error("worker size differs from the reviewed artifact")
    worker = args.worker.read_bytes()
    if hashlib.sha256(worker).hexdigest() != expected:
        parser.error("worker digest differs from the reviewed artifact")

    files = {
        "LICENSE": ((ROOT / "LICENSE").read_bytes(), 0o644),
        "boot-completed.sh": ((FIXTURE / "boot-completed.sh").read_bytes(), 0o755),
        "customize.sh": ((FIXTURE / "customize.sh").read_bytes(), 0o644),
        "earkey-worker": (worker, 0o500),
        "module.prop": ((FIXTURE / "module.prop").read_bytes(), 0o644),
        "worker.sha256": ((FIXTURE / "worker.sha256").read_bytes(), 0o644),
        "service.sh": ((FIXTURE / "service.sh").read_bytes(), 0o755),
        "skip_mount": ((FIXTURE / "skip_mount").read_bytes(), 0o644),
    }
    with ZipFile(args.output, "x", compression=ZIP_STORED) as archive:
        for name, (data, mode) in sorted(files.items()):
            member = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            member.create_system = 3
            member.external_attr = (stat.S_IFREG | mode) << 16
            archive.writestr(member, data)


if __name__ == "__main__":
    main()
