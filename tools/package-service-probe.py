#!/usr/bin/env python3
"""Package the exact Stage 4 ELF for the reference manager's native installer."""

import argparse
import hashlib
from pathlib import Path
import stat
from zipfile import ZIP_STORED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "resukisu-service-preflight"
PROBE_SIZE = 342_224


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probe", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    expected = (FIXTURE / "probe.sha256").read_text(encoding="ascii").strip()
    if args.probe.stat().st_size != PROBE_SIZE:
        parser.error("probe size differs from the reviewed artifact")
    probe = args.probe.read_bytes()
    if hashlib.sha256(probe).hexdigest() != expected:
        parser.error("probe digest differs from the reviewed artifact")

    files = {
        "LICENSE": ((ROOT / "LICENSE").read_bytes(), 0o644),
        "boot-completed.sh": ((FIXTURE / "boot-completed.sh").read_bytes(), 0o755),
        "customize.sh": ((FIXTURE / "customize.sh").read_bytes(), 0o644),
        "earkey-probe": (probe, 0o500),
        "module.prop": ((FIXTURE / "module.prop").read_bytes(), 0o644),
        "probe.sha256": ((FIXTURE / "probe.sha256").read_bytes(), 0o644),
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
