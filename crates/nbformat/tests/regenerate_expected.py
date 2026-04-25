#!/usr/bin/env python3
"""Regenerate tests/notebooks/expected/ from the Python nbformat writer.

For every .ipynb fixture under tests/notebooks/ that Python nbformat can read
and write without error, we write its canonical form to
tests/notebooks/expected/<name>.ipynb. The Rust conformance test
`test_matches_python_oracle_output` then parses each fixture with the nbformat
crate, serializes it, and asserts byte-for-byte equality against the expected
file. That check is skipped for fixtures without an expected companion (invalid
notebooks, v3 notebooks that need upgrading, etc.).

Run manually after adding fixtures or bumping the Python `nbformat` dependency:

    python3 tests/regenerate_expected.py

Requires: `pip install nbformat`.
"""

from __future__ import annotations

import json
import pathlib
import sys

try:
    import nbformat
except ImportError:
    sys.stderr.write(
        "error: Python package `nbformat` is required.\ninstall with: pip install nbformat\n"
    )
    sys.exit(1)


def main() -> int:
    here = pathlib.Path(__file__).resolve().parent
    fixtures_dir = here / "notebooks"
    expected_dir = fixtures_dir / "expected"
    expected_dir.mkdir(exist_ok=True)

    written: list[str] = []
    skipped: list[tuple[str, str]] = []

    for path in sorted(fixtures_dir.glob("*.ipynb")):
        # Skip fixtures whose filename marks them as invalid — the Rust crate
        # rejects these on parse, so a byte-for-byte oracle is not meaningful.
        if path.name.startswith("invalid"):
            skipped.append((path.name, "intentionally invalid (Rust crate rejects)"))
            continue

        try:
            raw_nb = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            raw_nb = {}
        if raw_nb and (raw_nb.get("nbformat") != 4 or raw_nb.get("nbformat_minor", 0) != 5):
            skipped.append(
                (
                    path.name,
                    f"not v4.5 (nbformat={raw_nb.get('nbformat')}.{raw_nb.get('nbformat_minor')})",
                )
            )
            continue
        if any("id" not in cell for cell in raw_nb.get("cells", [])):
            skipped.append((path.name, "missing cell ids (Python fills nondeterministic ids)"))
            continue

        try:
            # as_version=nbformat.NO_CONVERT preserves the original nbformat
            # version so we only regenerate notebooks that are valid at their
            # declared version. Upgrading would make the test drift from the
            # Rust crate's read path.
            nb = nbformat.read(str(path), as_version=nbformat.NO_CONVERT)
            if nb.get("nbformat") != 4 or nb.get("nbformat_minor", 0) != 5:
                # The Rust crate requires exactly v4.5 for serialize_notebook.
                skipped.append(
                    (
                        path.name,
                        f"not v4.5 (nbformat={nb.get('nbformat')}.{nb.get('nbformat_minor')})",
                    )
                )
                continue
            nbformat.validate(nb)
        except Exception as exc:
            skipped.append((path.name, f"{type(exc).__name__}: {exc}"))
            continue

        target = expected_dir / path.name
        with target.open("w", encoding="utf-8") as fh:
            nbformat.write(nb, fh)
        written.append(path.name)

    print(f"Wrote {len(written)} expected files to {expected_dir.relative_to(here.parent)}:")
    for name in written:
        print(f"  + {name}")
    if skipped:
        print(f"\nSkipped {len(skipped)} fixtures:")
        for name, reason in skipped:
            print(f"  - {name}: {reason}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
