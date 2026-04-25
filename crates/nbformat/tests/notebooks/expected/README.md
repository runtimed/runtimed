# Python `nbformat.write` oracle fixtures

This directory contains the byte-for-byte expected output of Python's
`nbformat.write` for a subset of the fixtures in `../`. The conformance test
`test_matches_python_oracle_output` parses each original fixture with the Rust
`nbformat` crate, serializes it, and asserts equality against its companion in
this directory.

Regenerate after adding new fixtures or upgrading the Python `nbformat`
package:

```
python3 tests/regenerate_expected.py
```

Fixtures are skipped by the regenerator in these cases:

- not v4.5 on disk (Python's reader auto-upgrades; Rust serializes only v4.5)
- filename starts with `invalid` (the Rust crate rejects these on parse)
- Python validation fails
- the fixture contains binary MIME outputs (`image/png`, etc.) — the current
  `jupyter-protocol` media serializer splits those into a list of lines,
  while Python emits a single string. This is a separate serialize-path
  divergence tracked independently; once fixed, the binary-filter in the
  regenerator can be removed and more fixtures will be covered.
