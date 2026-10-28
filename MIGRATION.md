# ZeroMQ 0.7 and WebSocket 0.35 migration

The next candidate releases are `jupyter-zmq-client` 3.0.0,
`runtimelib` 5.0.0, `jupyter-websocket-client` 3.0.0,
`jupyter-ysync` 0.3.0, and `ollama-kernel` 1.2.4.
`jupyter-protocol` remains at 2.0.3 and `nbformat` at 3.0.0.

## ZeroMQ public types

Update direct `zeromq` dependencies to `0.7` alongside `jupyter-zmq-client = "3"`
(or the deprecated `runtimelib = "5"` shim). Socket aliases, `Connection<S>`
trait bounds, `PeerIdentity`, `RuntimeError::ZmqError`, and subscription message
arguments expose ZeroMQ types; values from 0.6 and 0.7 cannot be mixed.
ZeroMQ's `SocketBackend::socket_options` now returns `&Arc<SocketOptions>`.
Windows IPC no longer depends on `win_uds`; the adapter lives in ZeroMQ.

Runtime and crypto features retain their existing defaults and explicit choices.
The dispatcher backend still transitively uses async-std. ZeroMQ 0.7 requires
Rust 1.89; this workspace continues to build and test with Rust 1.90.

## WebSocket public types

Use `jupyter-websocket-client = "3"` and align direct `async-tungstenite`
dependencies to `0.35`, including compatible Tokio/TLS features. This applies
when constructing `JupyterWebSocket::inner`. `RemoteServer::connect_to_kernel` manages stream construction for you.
Y-sync's optional client shares this dependency; its transport fields are private,
so this WebSocket update alone preserves its public API.

## Optional Python bindings

Y-sync 0.3.0 also incorporates #327: the optional `python` feature now uses
PyO3 0.29.3 or a compatible patch release. Rust callers passing `Python`,
`Bound`, or `PyResult` values through the public `jupyter_ysync::python` module
must align direct PyO3 dependencies to 0.29. This dependency type change requires
the new pre-1.0 compatibility series even though the Python methods are unchanged.
The extension explicitly retains its GIL requirement. Python distribution
metadata remains separate; this release does not publish a wheel to PyPI.

## Dependency resolution and TLS

Update application lockfiles when adopting these releases. The reviewed workspace
and packaged lockfiles use `rustls-webpki` 0.103.16 and `webpki-roots` 1.0.11.
Libraries do not impose their lockfiles on consumers; verify the application's
resolved TLS graph separately. ZeroMQ itself has no TLS/WebPKI dependency.

The following section documents the preceding release for older consumers.

# October 2026 release migration

The prepared release train is `jupyter-protocol` 2.0.3,
`jupyter-zmq-client` 2.0.0, `runtimelib` 4.0.0, and `ollama-kernel` 1.2.2.
These versions are candidates until the reviewed release commit is published.
The Ollama binary patch adopts the updated transport and protocol dependencies.

## Listener and feature changes

With `async-dispatcher-runtime`, `peek_ports_with_listeners()` returns
`(Vec<u16>, Vec<smol::net::TcpListener>)`. Replace explicit
`async_std::net::TcpListener` annotations/imports with `smol::net::TcpListener`
from Smol 2. Code that only holds and drops inferred listeners needs no type
change. Tokio callers retain `tokio::net::TcpListener`.

The direct `async-std` dependency and implicit feature are removed. This is not
an entirely async-std-free graph: `zeromq` 0.6 still enables it for its dispatcher
backend.

Defaults now provide Tokio plus ring. Applications using a different runtime
or crypto backend must disable defaults, for example:

```toml
jupyter-zmq-client = { version = "2", default-features = false, features = ["async-dispatcher-runtime", "aws-lc-rs"] }
smol = "2"
```

Select exactly one runtime (`tokio-runtime` or `async-dispatcher-runtime`) and
one crypto backend (`ring` or `aws-lc-rs`). `--all-features` is unsupported.
`runtimelib` 4 forwards the same API and features; new code should depend on
`jupyter-zmq-client` directly.

## Protocol compatibility

`jupyter-protocol` 2.0.3 accepts a CodeMirror object with a `name` and no
`version`, as emitted by Deno. It maps that object to `CodeMirrorMode::Simple`.
The existing public enum variants and `version: usize` fields are unchanged.
It is a patch within the protocol 2 series, not the proposed protocol 3 redesign.

## Zed and other consumers of older major versions

Zed main at `24a2823456f16e36bf32df510fb5c43fee7a2565` uses runtimelib and
jupyter-protocol 1.4.0, jupyter-websocket-client 1.1.0, and nbformat 1.2.0.
Publishing this train does not change those pins. Its Deno parsing report
[#59617](https://github.com/zed-industries/zed/issues/59617) is relevant to the
unreleased protocol fix. Its dependency dashboard lists the existing major
upgrades as pending approval, which does not establish an upstream blocker for
every old pin.

Consumers moving from those versions must coordinate protocol types across
the transport, WebSocket, and notebook crates. Mixing protocol 1 and protocol 2
types creates distinct Rust types. Also review earlier published breaking
changes: move `runtimelib` media/data-table imports to `jupyter_protocol`, and
handle the non-exhaustive `nbformat::Notebook` variants, including quirks mode.
Zed already uses Smol listeners locally and explicitly disables default features
for its dispatcher/AWS-LC pair; preserve that configuration.

Separate dependency PRs may update WebSocket and Python integrations. They are
not required to publish this train. `JupyterWebSocket::inner` exposes an
async-tungstenite stream: callers constructing it must align their direct
async-tungstenite version and features with the client. A move to the bounded
0.35 series belongs in a separate WebSocket major release. The optional Python
module similarly exposes PyO3 types and needs its own pre-1.0 breaking release
when upgraded.
