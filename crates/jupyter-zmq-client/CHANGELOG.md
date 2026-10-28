# Changelog

All notable changes to `jupyter-zmq-client` will be documented in this file.

## [Unreleased]

## [3.0.1] - 2026-10-28

- Reject multipart messages missing the signature frame or any of the four JSON frames before HMAC verification, returning an error instead of panicking (#332). This applies to signed and unsigned connections and preserves valid-message handling.

## [3.0.0] - 2026-10-28

- Upgrade to ZeroMQ 0.7. Public sockets, message/error types, trait bounds, and PeerIdentity now belong to the 0.7 series; update direct zeromq dependencies to match. Windows IPC uses the internal socket2/async-io adapter instead of win_uds. Runtime and crypto feature selection is unchanged.

## [2.0.0] - 2026-10-27

### Breaking

- Under `async-dispatcher-runtime`, `peek_ports_with_listeners()` now returns
  `smol::net::TcpListener` values instead of `async_std::net::TcpListener` (#325).
  Update explicit type annotations and direct listener imports to Smol 2. The
  Tokio return type is unchanged. The direct `async-std` dependency and its
  implicit Cargo feature are removed; `zeromq` still uses it transitively.
- Default features now select `tokio-runtime` and `ring`, so standalone packages
  compile with their defaults. Alternative runtime or crypto users must set
  `default-features = false` and select one runtime and one crypto backend.

### Fixed

- Exercise all four supported runtime/crypto combinations in CI, including
  signed message round trips and tamper rejection with AWS-LC.
- Check the Smol listener return type and port reservation lifetime.

## [1.0.1] - 2026-06-27

## [1.0.0] - 2026-05-05

Initial release under the new name. Previously published as [`runtimelib`](https://crates.io/crates/runtimelib) up through `2.0.0`; `runtimelib` now re-exports this crate as a deprecated shim.

### Renamed

- `runtimelib` → `jupyter-zmq-client`. Update `Cargo.toml` to depend on `jupyter-zmq-client` directly; `use runtimelib::…` becomes `use jupyter_zmq_client::…`. See [`crates/runtimelib/CHANGELOG.md`](../runtimelib/CHANGELOG.md) for prior history.

### Added

- `data_dirs_with_jupyter_paths()`, `list_kernelspecs_with_jupyter_paths()`, and `find_kernelspec_with_jupyter_paths()` to surface kernels installed in Python virtualenvs by augmenting the static directory list with the `data` paths reported by `jupyter --paths --json`. Falls back to the static dirs when `jupyter` is unavailable. Fixes #304.

### Changed

- `RuntimeError::KernelNotFound` now carries a `searched_paths: Vec<PathBuf>` field so callers can render the directories that were searched alongside the kernels that were discoverable. **Breaking change** for any consumer constructing the variant directly; consumers that only pattern-match are unaffected because `RuntimeError` is `#[non_exhaustive]`.
- Bump `zeromq` from `0.6.0-pre.2` to `0.6.0` (stable).
