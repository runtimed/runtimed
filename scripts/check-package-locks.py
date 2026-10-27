#!/usr/bin/env python3
"""Reject packaged dependencies outside the reviewed workspace resolution."""

import argparse
from pathlib import Path
import sys
import tarfile
import tomllib


def check_archive(archive, workspace):
    with tarfile.open(archive, "r:gz") as package:
        contents = package.extractfile(f"{archive.stem}/Cargo.lock")
        if contents is None:
            raise ValueError(f"{archive}: missing Cargo.lock")
        packaged = tomllib.loads(contents.read().decode("utf-8"))["package"]
    errors = []
    for dependency in packaged:
        name, version = dependency["name"], dependency["version"]
        candidates = [p for p in workspace if p["name"] == name and p["version"] == version]
        # Workspace packages acquire a registry source and checksum when packaged.
        if not any(
            "source" not in p
            or (p.get("source") == dependency.get("source")
                and p.get("checksum") == dependency.get("checksum"))
            for p in candidates
        ):
            errors.append(f"{archive}: {name} {version} is outside the reviewed lockfile")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lockfile", type=Path, default=Path("Cargo.lock"))
    parser.add_argument("archives", type=Path, nargs="+")
    args = parser.parse_args()
    workspace = tomllib.loads(args.lockfile.read_text())["package"]
    errors = [error for archive in args.archives for error in check_archive(archive, workspace)]
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Verified {len(args.archives)} archive lockfiles against {args.lockfile}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
