# Releasing

We use `cargo-release` within the workspace. Always run dry first (the default), then run with `--execute` when ready.

## Dependency Order

The workspace has this dependency structure:

```
jupyter-protocol (no internal deps)
    ↓
jupyter-serde, nbformat, jupyter-websocket-client, runtimelib
    ↓
ollama-kernel, sidecar, runt-cli
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

### 3. Binaries (last, depend on runtimelib)

> [!WARNING]
> Runtimelib _must_ be published before these.

These use different async runtimes and must be released individually:

```
cargo release -p ollama-kernel <patch|minor|major>
```

```
cargo release -p sidecar -p runt-cli <patch|minor|major>
```

## Targeted Patch Releases

If changes only touch specific crates, you can release just those (and their dependents if needed). Check what changed since last release:

```
git log --oneline <last-tag>..HEAD --name-only | grep -E '^crates/' | sort -u
```

## Python Package (runtimed)

The Python package bundles the `runt` binary and is released separately from the Rust crates.

### 1. Bump the version

Edit `python/runtimed/pyproject.toml` and update the `version` field.

### 2. Create a PR

Open a PR with the version bump, get it reviewed and merged.

### 3. Tag and push

```
git tag python-v<version>
git push origin python-v<version>
```

The `python-package.yml` workflow triggers on `python-v*` tags and will:
- Build wheels for macOS (arm64 + x64) and Linux (x64)
- Publish to PyPI via trusted publishing
- Create a GitHub release with wheels and `runt` binaries
