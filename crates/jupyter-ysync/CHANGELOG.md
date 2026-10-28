# Changelog

## [Unreleased]

## [0.3.0] - 2026-10-28

### Breaking

- Upgrade the optional Python bindings to PyO3 0.29.3, including fixes for
  RUSTSEC-2026-0176 and RUSTSEC-2026-0177. Rust callers of the public `python`
  module must align their PyO3 dependency to 0.29. The extension continues to
  require the GIL; free-threaded execution has not been validated.

### Changed

- Test installed Python bindings on Python 3.10 and 3.14. Python distribution
  metadata is separate from this Rust crate release; no PyPI release is included.

- Use jupyter-websocket-client 3 and share async-tungstenite 0.35 for the optional client feature, removing the separate 0.32 dependency. These transport types remain private; the WebSocket update itself does not change the public API.
