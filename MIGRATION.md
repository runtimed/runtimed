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
