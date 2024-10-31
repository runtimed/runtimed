# Releasing

We push releases using `cargo-release` within the workspace, which keeps dependent packages
up to date and published together.

## Set up release

```
cargo release --workspace minor # alternatively major or patch
```

If that's looking good, run it again with `--execute`.

## Example release

```
$ cargo release --workspace patch

Upgrading jupyter-serde from 0.2.0 to 0.2.1
 Updating jupyter-websocket-client's dependency from 0.2.0 to 0.2.1
 Updating nbformat's dependency from 0.2.0 to 0.2.1
 Updating runtimelib's dependency from 0.2.0 to 0.2.1
Upgrading nbformat from 0.3.1 to 0.3.2
Upgrading runtimelib from 0.16.0 to 0.16.1
 Updating jupyter-websocket-client's dependency from 0.16.0 to 0.16.1
Upgrading jupyter-websocket-client from 0.2.0 to 0.2.1
Publishing jupyter-serde
 Updating crates.io index
Packaging jupyter-serde v0.2.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/jupyter-serde)
 Packaged 5 files, 23.3KiB (5.6KiB compressed)
Uploading jupyter-serde v0.2.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/jupyter-serde)
warning: aborting upload due to dry run
Publishing nbformat
 Updating crates.io index
Packaging nbformat v0.3.1 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/nbformat)
 Packaged 34 files, 288.3KiB (63.1KiB compressed)
Uploading nbformat v0.3.1 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/nbformat)
warning: aborting upload due to dry run
Publishing runtimelib
 Updating crates.io index
Packaging runtimelib v0.16.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/runtimelib)
 Packaged 14 files, 87.8KiB (19.6KiB compressed)
Uploading runtimelib v0.16.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/runtimelib)
warning: aborting upload due to dry run
Publishing jupyter-websocket-client
 Updating crates.io index
Packaging jupyter-websocket-client v0.2.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/jupyter-websocket-client)
 Updating crates.io index
 Packaged 6 files, 62.3KiB (16.4KiB compressed)
Uploading jupyter-websocket-client v0.2.0 (/Users/kylekelley/code/src/github.com/runtimed/runtimed/crates/jupyter-websocket-client)
warning: aborting upload due to dry run
  Pushing Pushing jupyter-serde-v0.2.1, jupyter-websocket-client-v0.2.1, main, nbformat-v0.3.2, runtimelib-v0.16.1 to origin
warning: aborting release due to dry run; re-run with `--execute`
```
