# jupyter-zmq-client

`jupyter-zmq-client` is a Rust library for interacting with Jupyter kernels natively, over ZeroMQ.

This crate was previously published as [`runtimelib`](https://crates.io/crates/runtimelib). `runtimelib` now re-exports this crate for backwards compatibility and is deprecated.

## Installation

`jupyter-zmq-client` lets you pick which async runtime you want to use. If you're using tokio, include the `tokio-runtime` flag. For `async-dispatcher` users (AKA GPUI devs), use `async-dispatcher-runtime`. The async dispatcher runtime is also compatible with smol/async-std users.

### Tokio Users

```toml
[dependencies]
jupyter-zmq-client = { version = "1", features = ["tokio-runtime"] }
```

### Async-dispatcher Users

```toml
[dependencies]
jupyter-zmq-client = { version = "1", features = ["async-dispatcher-runtime"] }
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
