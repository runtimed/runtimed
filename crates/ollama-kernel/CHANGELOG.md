# Changelog

## [1.2.3] - 2026-10-27

- Refresh the packaged dependency resolution to include patched h2, Rustls,
  rustls-webpki, rand, and scc releases. The default TLS backend remains native TLS.
- Track the workspace lockfile and check packaged resolutions against it so
  publishing from another machine preserves the reviewed dependency versions.
