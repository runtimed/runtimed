// todo: move all of jupyter-serde into jupyter-protocol and make that the main
// crate that native clients, native kernels, and the websocket client depend on
pub mod messaging;
pub use messaging::*;

pub mod connection_info;
pub use connection_info::ConnectionInfo;

mod time;

mod execution_count;
pub use execution_count::*;

mod kernelspec;
pub use kernelspec::*;

pub mod media;
pub use media::*;

use async_trait::async_trait;
use futures::{Sink, Stream};

#[async_trait]
pub trait JupyterConnection:
    Sink<JupyterMessage> + Stream<Item = Result<JupyterMessage, anyhow::Error>>
{
}
