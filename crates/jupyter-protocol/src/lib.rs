pub use jupyter_serde::*;

pub mod messaging;

pub use messaging::*;

use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait JupyterConnection {
    type Error: Error + Send + Sync + 'static;
    type SendHalf: JupyterSender<Error = Self::Error>;
    type ReceiveHalf: JupyterReceiver<Error = Self::Error>;

    async fn send(&mut self, message: JupyterMessage) -> Result<(), Self::Error>;
    async fn receive(&mut self) -> Result<JupyterMessage, Self::Error>;
    fn split(self) -> (Self::SendHalf, Self::ReceiveHalf);
}

#[async_trait]
pub trait JupyterSender {
    type Error: Error + Send + Sync + 'static;

    async fn send(&mut self, message: JupyterMessage) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait JupyterReceiver {
    type Error: Error + Send + Sync + 'static;

    async fn receive(&mut self) -> Result<JupyterMessage, Self::Error>;
}

#[async_trait]
pub trait JupyterClient: JupyterConnection {
    async fn connect(&mut self) -> Result<(), Self::Error>;
    async fn disconnect(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait JupyterKernel: JupyterConnection {
    async fn start(&mut self) -> Result<(), Self::Error>;
    async fn shutdown(&mut self) -> Result<(), Self::Error>;
}
