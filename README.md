## runtimed

![lilrunt](https://github.com/runtimed/runtimed/assets/836375/f5d36136-5154-4c2c-b968-4354c29670b1)

The runtimed project is tooling for working with Jupyter. If you want to interact directly with jupyter kernels, use `runtimelib`. Everything else is in development.

### Goal

The goal of `runtimed` is to provide simple, easy to use, and powerful access to interactive computing. We want to enable a new generation of builders to:

- Create new notebook applications
- Create new kinds of REPLs
- Allow large language models to reason about code and data

## Getting Started with `runtimelib`

```
cargo install runtimelib
```

### Asynchronous dispatch options

By default, runtimelib uses tokio. However, the [async-dispatcher](https://github.com/zed-industries/async-dispatcher) runtime can be selected at compile time with:

```bash
cargo build --feature async-dispatch-runtime
```

This will allow you to build GPUI apps with runtimelib.
