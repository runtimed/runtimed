# Changelog

All notable changes to `runtimelib` will be documented in this file.

## [Unreleased]

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
