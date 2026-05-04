//! # runtimelib (deprecated — renamed to [`jupyter_zmq_client`])
//!
//! This crate has been renamed to [`jupyter-zmq-client`](https://crates.io/crates/jupyter-zmq-client).
//!
//! `runtimelib` is now a thin re-export shim published for backwards compatibility.
//! New code should depend on `jupyter-zmq-client` directly and import from
//! `jupyter_zmq_client::…` instead of `runtimelib::…`.
//!
//! ## Migration
//!
//! ```toml
//! # before
//! runtimelib = { version = "2", features = ["tokio-runtime"] }
//!
//! # after
//! jupyter-zmq-client = { version = "1", features = ["tokio-runtime"] }
//! ```
//!
//! ```rust,ignore
//! // before
//! use runtimelib::{create_client_shell_connection, list_kernelspecs};
//!
//! // after
//! use jupyter_zmq_client::{create_client_shell_connection, list_kernelspecs};
//! ```
//!
//! The feature flags (`tokio-runtime`, `async-dispatcher-runtime`, `ring`,
//! `aws-lc-rs`, `test-kernel`) behave identically and are forwarded to
//! `jupyter-zmq-client` unchanged.
#![deprecated(
    since = "3.0.0",
    note = "`runtimelib` has been renamed to `jupyter-zmq-client`. Depend on `jupyter-zmq-client` directly; this crate is a thin re-export shim."
)]

pub use jupyter_zmq_client::*;
