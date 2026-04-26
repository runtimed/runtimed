#![doc = include_str!("../README.md")]

pub mod kernelspec;
pub use kernelspec::*;

pub mod dirs;
pub use dirs::*;

mod error;
pub use error::{Result, RuntimeError};

#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub mod connection;
#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub use connection::*;

#[cfg(feature = "test-kernel")]
pub mod test_kernel;
#[cfg(feature = "test-kernel")]
pub use test_kernel::*;
