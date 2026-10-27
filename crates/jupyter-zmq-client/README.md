# jupyter-zmq-client

`jupyter-zmq-client` is a Rust library for interacting with Jupyter kernels natively, over ZeroMQ.

This crate was previously published as [`runtimelib`](https://crates.io/crates/runtimelib). `runtimelib` now re-exports this crate for backwards compatibility and is deprecated.

## Installation

The default features are `tokio-runtime` and `ring`. Select exactly one async
runtime and one HMAC backend. For `async-dispatcher` users (including GPUI and
smol applications), disable defaults and enable `async-dispatcher-runtime` with
either `ring` or `aws-lc-rs`.

| Runtime | HMAC backend | Features with `default-features = false` |
| --- | --- | --- |
| Tokio | ring | `tokio-runtime`, `ring` |
| Tokio | AWS-LC | `tokio-runtime`, `aws-lc-rs` |
| async-dispatcher / smol | ring | `async-dispatcher-runtime`, `ring` |
| async-dispatcher / smol | AWS-LC | `async-dispatcher-runtime`, `aws-lc-rs` |

Enabling both runtimes or both crypto backends is unsupported. In particular,
`--all-features` is not a supported build. Cargo unifies features within a
dependency graph, so all users of this crate must agree on the runtime and
backend. ZeroMQ can emit its own compilation errors for incompatible runtime
selections before this crate's diagnostics are reached.

### Tokio Users

```toml
[dependencies]
jupyter-zmq-client = { version = "1", features = ["tokio-runtime"] }
```

### Async-dispatcher Users

```toml
[dependencies]
jupyter-zmq-client = { version = "1", default-features = false, features = ["async-dispatcher-runtime", "ring"] }
```

### AWS-LC Users

```toml
[dependencies]
jupyter-zmq-client = { version = "1", default-features = false, features = ["tokio-runtime", "aws-lc-rs"] }
```

## Key Features

- **Jupyter Kernel Management**: Discover, start, and manage Jupyter kernels.
- **Messaging Protocol**: Implement Jupyter's wire protocol for communication with kernels over ZeroMQ.
- **Flexible Async Runtime**: Support for both Tokio and async-dispatcher runtimes.

## Message Types

For Jupyter message types, traits, and media types, use the [`jupyter-protocol`](https://crates.io/crates/jupyter-protocol) crate directly:

```toml
[dependencies]
jupyter-protocol = "2"
```

## Documentation

For more detailed information about the API and its usage, please refer to the [API documentation](https://docs.rs/jupyter-zmq-client).

## License

`jupyter-zmq-client` is distributed under the terms of the BSD 3-Clause license. See [LICENSE](../../LICENSE) for details.
