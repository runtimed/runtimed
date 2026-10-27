# Changelog

All notable changes to `runtimelib` will be documented in this file.

## [Unreleased]

## [4.0.0] - 2026-10-27

### Breaking

- Re-export `jupyter-zmq-client` 2, including its Smol listener return type and
  default `tokio-runtime,ring` features. Alternative runtime/crypto combinations
  still work with `default-features = false` and explicit features.
- Prefer migrating directly to `jupyter-zmq-client` rather than this deprecated
  shim. See the workspace [migration guide](../../MIGRATION.md).

## [3.0.0] - 2026-05-05

### Deprecated

- **This crate has been renamed to [`jupyter-zmq-client`](https://crates.io/crates/jupyter-zmq-client).** `runtimelib` is now a thin re-export shim that forwards all items and feature flags to `jupyter-zmq-client`. Update your `Cargo.toml` and `use` statements; this shim will not receive further changes beyond what flows through from `jupyter-zmq-client`.



## [2.0.0] - 2026-04-26

## [1.6.0] - 2026-04-12

### Added

- `peek_ports_with_listeners()` to hold TcpListeners across kernel spawn, closing the port reuse race window. (#296)

### Internal

- Add tokio mutex lint integration test. (#297)

## [1.5.0] - 2026-03-06

## [1.4.0] - 2026-02-25

### Fixed

- Exclude buffers from HMAC signature. (#280)

## [1.3.0] - 2026-02-20

### Added

- `TestKernel` for testing Jupyter frontends. (#270)

## [1.2.0] - 2026-02-11

### Added

- Stdin-aware connection API and `with_channel()` builder.

## [1.1.0] - 2026-02-09

### Added

- Initial `KernelClient` implementation.
- Support for `runt start` command.
- Expose `split()` for Jupyter shell channel.

### Fixed

- Avoid breaking changes to paths APIs.

## [1.0.0] - 2026-01-07

Stable release of the Jupyter runtime library. See git history for earlier changes.
