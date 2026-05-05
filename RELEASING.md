# Releasing

We use `cargo-release` within the workspace. Always run dry first (the default), then run with `--execute` when ready.

## Dependency Order

The workspace has this dependency structure:

```
jupyter-protocol (no internal deps)
    ↓
nbformat, jupyter-websocket-client, jupyter-zmq-client
    ↓
runtimelib (deprecated shim → jupyter-zmq-client)
    ↓
jupyter-ysync (depends on jupyter-protocol, nbformat; optionally jupyter-websocket-client)
    ↓
ollama-kernel (depends on jupyter-zmq-client)
```

`mybinder` is standalone with no internal dependencies.

> [!IMPORTANT]
> Each crate's dependencies must already be published to crates.io before it can be published. `runtimelib` declares `jupyter-zmq-client = "1"` as a hard version requirement, so `cargo publish -p runtimelib` will fail until `jupyter-zmq-client` is live. Same for `ollama-kernel` → `jupyter-zmq-client`, and everything → `jupyter-protocol`.

## Release Commands

### 1. jupyter-protocol (first, everything depends on it)

```
cargo release -p jupyter-protocol <patch|minor|major>
```

### 2. Protocol consumers

These all depend on `jupyter-protocol` and can be released together:

```
cargo release -p nbformat -p jupyter-websocket-client <patch|minor|major>
```

`jupyter-zmq-client` requires a feature flag when publishing:

```
cargo release -p jupyter-zmq-client --features tokio-runtime <patch|minor|major>
```

> [!NOTE]
> **First publish of `jupyter-zmq-client 1.0.0`**: since the crate has never been released, use the `release` level (which publishes the current in-tree version without bumping) rather than `major`/`minor`/`patch`:
>
> ```
> cargo release -p jupyter-zmq-client --features tokio-runtime release
> ```

### 3. runtimelib (deprecated shim)

> [!WARNING]
> `jupyter-zmq-client` _must_ be published before this — `runtimelib` depends on it by version number.

`runtimelib` is a thin re-export of `jupyter-zmq-client`. Only bump it when `jupyter-zmq-client` has been published and you need the shim to surface the new version. Most changes do not require a `runtimelib` release.

```
cargo release -p runtimelib --features tokio-runtime <patch|minor|major>
```

> [!NOTE]
> **First publish of `runtimelib 3.0.0`** (the shim): use `release` rather than `major`:
>
> ```
> cargo release -p runtimelib --features tokio-runtime release
> ```

### 4. jupyter-ysync

> [!WARNING]
> jupyter-protocol, nbformat, and jupyter-websocket-client must be published before this.

```
cargo release -p jupyter-ysync <patch|minor|major>
```

### 5. ollama-kernel

> [!WARNING]
> `jupyter-zmq-client` _must_ be published before this.

```
cargo release -p ollama-kernel <patch|minor|major>
```

## Targeted Patch Releases

If changes only touch specific crates, you can release just those (and their dependents if needed). Check what changed since last release:

```
git log --oneline <last-tag>..HEAD --name-only | grep -E '^crates/' | sort -u
```

## Changelog Convention

Each crate's `release.toml` runs a `pre-release-replacement` that promotes the `## [Unreleased]` section header, inserting a new `## [<version>] - <date>` heading below it. Add in-flight notes under `## [Unreleased]`; **do not** pre-populate dated release headers — `cargo-release` owns those.

## CLI & Desktop Tools

For releases of `runt`, `sidecar`, and the Python `runtimed` package, see the [runtimed/runt](https://github.com/runtimed/runt) repository.
