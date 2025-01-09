# Releasing

We push releases using `cargo-release` within the workspace, which keeps dependent packages up to date and published together. Make sure to run it dry first then run with `--execute` afterward.

Since there are troubles with features being incompatible between packages (namely tokio vs async-std), we have to release packages in this dependency order:

```
cargo release -p jupyter-serde -p nbformat -p jupyter-protocol -p jupyter-websocket-client minor
```

> [!WARNING]
> Runtimelib _must_ be shipped before Ollama kernel and sidecar

Runtimelib requires at least one feature flag being selected when publishing. For now, this uses `tokio-runtime`.

```
cargo release -p runtimelib --features tokio-runtime minor
```

Binaries that rely on `runtimelib` use different async runtimes and also don't have flags on themselves, so you must ship each individually.

```
cargo release -p ollama-kernel minor
```

```
cargo release -p sidecar -p runt-cli minor
```
