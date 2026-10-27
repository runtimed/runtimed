#![doc = include_str!("../README.md")]

#[cfg(all(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
compile_error!("select only one runtime: `tokio-runtime` or `async-dispatcher-runtime`; disable default features for async-dispatcher");
#[cfg(not(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime")))]
compile_error!("select a runtime: `tokio-runtime` or `async-dispatcher-runtime`");
#[cfg(all(feature = "ring", feature = "aws-lc-rs"))]
compile_error!(
    "select only one crypto backend: `ring` or `aws-lc-rs`; disable default features for aws-lc-rs"
);
#[cfg(not(any(feature = "ring", feature = "aws-lc-rs")))]
compile_error!("select a crypto backend: `ring` or `aws-lc-rs`");

pub mod kernelspec;
pub use kernelspec::*;

pub mod dirs;
pub use dirs::*;

mod error;
pub use error::{Result, RuntimeError};

#[cfg(all(
    any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"),
    any(feature = "ring", feature = "aws-lc-rs"),
    not(all(feature = "ring", feature = "aws-lc-rs"))
))]
pub mod connection;
#[cfg(all(
    any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"),
    any(feature = "ring", feature = "aws-lc-rs"),
    not(all(feature = "ring", feature = "aws-lc-rs"))
))]
pub use connection::*;

#[cfg(feature = "test-kernel")]
pub mod test_kernel;
#[cfg(feature = "test-kernel")]
pub use test_kernel::*;
