## runtimed

![lilrunt](https://github.com/runtimed/runtimed/assets/836375/f5d36136-5154-4c2c-b968-4354c29670b1)

The runtimed project provides Rust libraries for working with Jupyter.

### Libraries

- [`jupyter-protocol`](./crates/jupyter-protocol): Core types for Jupyter messages, independent of the underlying transport
- [`jupyter-websocket-client`](./crates/jupyter-websocket-client): Connect to Jupyter servers, both local and remote, over WebSockets
- [`nbformat`](./crates/nbformat): Parse and work with Jupyter Notebooks
- [`runtimelib`](./crates/runtimelib): Interact natively with Jupyter kernels over ZeroMQ

### Reference Implementation

- [`ollama-kernel`](./crates/ollama-kernel): A Jupyter kernel for interacting with Ollama

### CLI & Desktop Tools

For the `runt` CLI and desktop applications (sidecar, notebook), see [runtimed/runt](https://github.com/runtimed/runt).

### Goal

The goal of `runtimed` is to provide Rust developers with simple, easy to use, and powerful access to interactive computing. We want to enable a new generation of builders to:

- Create new notebook applications
- Create new kinds of REPLs
- Allow large language models to reason about code and data
