use anyhow::{Context, Result};
use async_tungstenite::{
    tokio::connect_async, tokio::ConnectStream, tungstenite::Message, WebSocketStream,
};
use futures::{Sink, SinkExt as _, Stream, StreamExt};

use bytes::Bytes;
use jupyter_serde::messaging::{JupyterMessageContent, *};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

pub struct JupyterWebSocket {
    inner: WebSocketStream<ConnectStream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Shell,
    Control,
    Stdin,
    IOPub,
    Heartbeat,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct JupyterMessage {
    pub header: Value, // todo
    pub parent_header: Option<Value>,
    pub metadata: Value,
    pub content: JupyterMessageContent,
    #[serde(skip_serializing, skip_deserializing)]
    pub buffers: Vec<Bytes>,
    pub channel: Option<Channel>,
}

impl JupyterMessage {
    pub fn new(content: JupyterMessageContent, channel: Option<Channel>) -> Self {
        JupyterMessage {
            header: Value::Null,
            parent_header: None,
            metadata: Value::Null,
            content,
            buffers: Vec::new(),
            channel,
        }
    }
}

impl From<JupyterMessageContent> for JupyterMessage {
    fn from(content: JupyterMessageContent) -> Self {
        JupyterMessage::new(content, None)
    }
}

pub trait IntoJupyterMessage {
    fn into_jupyter_message(self) -> JupyterMessage;
    fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage;
}

macro_rules! impl_message_traits {
    ($($name:ident),*) => {
        $(
            impl IntoJupyterMessage for $name {
                #[doc = concat!("Create a new `JupyterMessage`, assigning the parent for a `", stringify!($name), "` message.\n")]
                ///
                /// This method creates a new `JupyterMessage` with the right content, parent header, and zmq identities, making
                /// it suitable for sending over ZeroMQ.
                ///
                /// # Example
                /// ```ignore
                /// use runtimelib::messaging::{JupyterMessage, JupyterMessageContent};
                ///
                /// let message = connection.recv().await?;
                ///
                #[doc = concat!("let child_message = ", stringify!($name), "{\n")]
                ///   // ...
                /// }.as_child_of(&message);
                ///
                /// connection.send(child_message).await?;
                /// ```
                #[must_use]
                fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage {
                    JupyterMessage::new(self.clone(), Some(parent))
                }

                #[doc = concat!("Create a new `JupyterMessage` for a `", stringify!($name), "`.\n\n")]
                /// This method creates a new `JupyterMessage` with the right content, parent header, and zmq identities, making
                /// it suitable for sending over ZeroMQ.
                #[must_use]
                fn into_jupyter_message(self) -> JupyterMessage {
                    JupyterMessage::new(self, None)
                }
            }

        )*
    };
}

impl_message_traits!(
    ClearOutput,
    CommClose,
    CommInfoReply,
    CommInfoRequest,
    CommMsg,
    CommOpen,
    CompleteReply,
    CompleteRequest,
    DebugReply,
    DebugRequest,
    DisplayData,
    ErrorOutput,
    ExecuteInput,
    ExecuteReply,
    ExecuteRequest,
    ExecuteResult,
    HistoryReply,
    HistoryRequest,
    InputReply,
    InputRequest,
    InspectReply,
    InspectRequest,
    InterruptReply,
    InterruptRequest,
    IsCompleteReply,
    IsCompleteRequest,
    KernelInfoRequest,
    ShutdownReply,
    ShutdownRequest,
    Status,
    StreamContent,
    UpdateDisplayData,
    UnknownMessage
);

impl Stream for JupyterWebSocket {
    type Item = Result<JupyterMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(msg))) => match msg {
                Message::Text(text) => Poll::Ready(Some(
                    serde_json::from_str(&text)
                        .context("Failed to parse JSON")
                        .and_then(|value| {
                            serde_json::from_value::<JupyterMessage>(value)
                                .context("Failed to create JupyterMessage")
                        }),
                )),
                _ => Poll::Ready(Some(Err(anyhow::anyhow!("Received non-text message")))),
            },
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Sink<JupyterMessage> for JupyterWebSocket {
    type Error = anyhow::Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready_unpin(cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, item: JupyterMessage) -> Result<(), Self::Error> {
        let message_str =
            serde_json::to_string(&item).context("Failed to serialize JupyterMessage")?;
        self.inner
            .start_send_unpin(Message::Text(message_str))
            .map_err(Into::into)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_flush_unpin(cx).map_err(Into::into)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_close_unpin(cx).map_err(Into::into)
    }
}

pub async fn connect(url: &str) -> Result<JupyterWebSocket> {
    let (ws_stream, _) = connect_async(url)
        .await
        .context("Failed to connect to WebSocket")?;
    Ok(JupyterWebSocket { inner: ws_stream })
}
