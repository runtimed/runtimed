# Changelog

## [Unreleased]

- Accept async-tungstenite 0.33 through 0.35 (`>=0.33, <0.36`). Applications constructing `JupyterWebSocket::inner` can stay on the 0.33 series. That keeps tungstenite 0.28 and its sha1 0.10 / digest 0.10 stack instead of pulling a second RustCrypto 0.11 line. CI tests both ends of the range.

## [3.0.0] - 2026-10-28

- Bound async-tungstenite to the 0.35 compatibility series. Applications constructing JupyterWebSocket::inner must align their direct dependency to 0.35. Correct the README to describe the supported Tokio runtime.
