# runtimelib

> [!WARNING]
> **`runtimelib` has been renamed to [`jupyter-zmq-client`](https://crates.io/crates/jupyter-zmq-client).**
>
> This crate is now a thin re-export shim kept for backwards compatibility. New code should depend on `jupyter-zmq-client` directly.

## Migration

```toml
# before
runtimelib = { version = "2", features = ["tokio-runtime"] }

# after
jupyter-zmq-client = { version = "1", features = ["tokio-runtime"] }
```

```rust,ignore
// before
use runtimelib::{create_client_shell_connection, list_kernelspecs};

// after
use jupyter_zmq_client::{create_client_shell_connection, list_kernelspecs};
```

All feature flags (`tokio-runtime`, `async-dispatcher-runtime`, `ring`, `aws-lc-rs`, `test-kernel`) are forwarded unchanged.

## History

For the changelog of this crate prior to the rename, see the [`runtimelib` CHANGELOG](./CHANGELOG.md). For ongoing development, see the [`jupyter-zmq-client` CHANGELOG](../jupyter-zmq-client/CHANGELOG.md).

## License

Distributed under the BSD 3-Clause license. See [LICENSE](../../LICENSE) for details.
