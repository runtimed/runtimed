# Changelog

All notable changes to `jupyter-zmq-client` will be documented in this file.

## [Unreleased]

Initial release under the new name. Previously published as [`runtimelib`](https://crates.io/crates/runtimelib) up through `2.0.0`; `runtimelib` now re-exports this crate as a deprecated shim.

### Renamed

- `runtimelib` → `jupyter-zmq-client`. Update `Cargo.toml` to depend on `jupyter-zmq-client` directly; `use runtimelib::…` becomes `use jupyter_zmq_client::…`. See [`crates/runtimelib/CHANGELOG.md`](../runtimelib/CHANGELOG.md) for prior history.

### Added

- `data_dirs_with_jupyter_paths()`, `list_kernelspecs_with_jupyter_paths()`, and `find_kernelspec_with_jupyter_paths()` to surface kernels installed in Python virtualenvs by augmenting the static directory list with the `data` paths reported by `jupyter --paths --json`. Falls back to the static dirs when `jupyter` is unavailable. Fixes #304.

### Changed

- `RuntimeError::KernelNotFound` now carries a `searched_paths: Vec<PathBuf>` field so callers can render the directories that were searched alongside the kernels that were discoverable. **Breaking change** for any consumer constructing the variant directly; consumers that only pattern-match are unaffected because `RuntimeError` is `#[non_exhaustive]`.
