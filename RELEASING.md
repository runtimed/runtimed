# Releasing

We use `cargo-release` within the workspace. Always run dry first (the default), then run with `--execute` when ready.

## Dependency Order

The workspace has this dependency structure:

```
jupyter-protocol (no internal deps)
    ↓
jupyter-serde, nbformat, jupyter-websocket-client, runtimelib
    ↓
ollama-kernel
```

`mybinder` is standalone with no internal dependencies.

## Release Commands

### 1. jupyter-protocol (first, everything depends on it)

```
cargo release -p jupyter-protocol <patch|minor|major>
```

### 2. Protocol consumers

These all depend on `jupyter-protocol` and can be released together:

```
cargo release -p jupyter-serde -p nbformat -p jupyter-websocket-client <patch|minor|major>
```

Runtimelib requires a feature flag when publishing:

```
cargo release -p runtimelib --features tokio-runtime <patch|minor|major>
```

### 3. ollama-kernel

> [!WARNING]
> Runtimelib _must_ be published before this.

```
cargo release -p ollama-kernel <patch|minor|major>
```

## Targeted Patch Releases

If changes only touch specific crates, you can release just those (and their dependents if needed). Check what changed since last release:

```
git log --oneline <last-tag>..HEAD --name-only | grep -E '^crates/' | sort -u
```

## CLI & Desktop Tools

For releases of `runt`, `sidecar`, and the Python `runtimed` package, see the [runtimed/runt](https://github.com/runtimed/runt) repository.
