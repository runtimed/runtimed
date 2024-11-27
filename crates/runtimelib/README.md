# runtimelib

`runtimelib` is a Rust library for interacting with Jupyter kernels and managing interactive computing environments. It provides a set of tools and abstractions to simplify the process of working with various programming language runtimes, enabling developers to build powerful applications that leverage the capabilities of Jupyter kernels.

## Introduction

runtimelib serves as the foundation for building interactive computing applications, REPLs, and notebook-like interfaces. It abstracts away the complexities of communicating with Jupyter kernels, allowing developers to focus on creating rich, interactive experiences for users.

Key features of runtimelib include:

- Easy integration with Jupyter kernels
- Asynchronous communication with kernels
- Support for multiple runtime environments
- Extensible architecture for custom kernel implementations

Whether you're building a new notebook application, creating a specialized REPL, or integrating interactive computing capabilities into your existing projects, runtimelib provides the tools and flexibility you need to get started quickly and efficiently.


## Installation

Runtimelib allows you to pick which async runtime you want to use. If you're using tokio, include the `tokio-runtime` flag. For `async-dispatcher` users (AKA GPUI devs), use `async-dispatcher-runtime`. The async dispatcher runtime is also compatible for smol/async-std users.

### Tokio Users

```toml
[dependencies]
runtimelib = { version = "0.22.0", features = ["tokio-runtime"] }
```

### Async-dispatcher Users

```toml
[dependencies]
runtimelib = { version = "0.22.0", features = ["async-dispatcher-runtime"] }
```

## Key Features

- **Jupyter Kernel Management**: Discover, start, and manage Jupyter kernels.
- **Messaging Protocol**: Implement the Jupyter messaging protocol for communication with kernels.
- **Runtime Management**: Create and manage runtime instances for interactive computing.
- **Flexible Async Runtime**: Support for both Tokio and async-dispatcher runtimes.
- **Media Handling**: Work with various media types used in Jupyter, including images, HTML, and more.

## Documentation

For more detailed information about the API and its usage, please refer to the [API documentation](https://docs.rs/runtimelib).

## Contributing

We welcome contributions to RuntimeLib! If you'd like to contribute, please:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Write tests for your changes
4. Implement your changes
5. Submit a pull request

Please make sure to update tests as appropriate and adhere to the existing coding style.

## License

RuntimeLib is distributed under the terms of both the MIT license and the Apache License (Version 2.0). See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
