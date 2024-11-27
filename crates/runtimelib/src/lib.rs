//! Runtimelib: A Rust library for interacting with Jupyter kernels and managing interactive computing environments.
//!
//! This library provides bindings to Jupyter kernels over ZeroMQ.

pub use jupyter_serde::media;
pub use jupyter_serde::media::*;
pub use jupyter_serde::ExecutionCount;

pub mod jupyter;
pub use jupyter::*;

#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub mod connection;
#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub use connection::*;
