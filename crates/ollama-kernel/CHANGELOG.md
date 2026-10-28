# Changelog

## [Unreleased]

## [1.2.4] - 2026-10-28

- Use jupyter-zmq-client 3 with ZeroMQ 0.7, including the internal Windows IPC adapter. Preserve the reviewed TLS dependency resolution in the published binary lockfile.

## [1.2.3] - 2026-10-27

- Refresh the packaged dependency resolution to include patched h2, Rustls,
  rustls-webpki, rand, and scc releases. The default TLS backend remains native TLS.
- Track the workspace lockfile and check packaged resolutions against it so
  publishing from another machine preserves the reviewed dependency versions.
