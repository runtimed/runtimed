use anyhow::{Context, Result};
use async_tungstenite::{
    tokio::connect_async, tokio::ConnectStream, tungstenite::Message, WebSocketStream,
};
use futures::{Sink, SinkExt as _, Stream, StreamExt};

use runtimelib::JupyterMessage;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

pub struct JupyterWebSocket {
    inner: WebSocketStream<ConnectStream>,
}

impl Stream for JupyterWebSocket {
    type Item = Result<JupyterMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(msg))) => match msg {
                Message::Text(text) => Poll::Ready(Some(
                    serde_json::from_str(&text)
                        .context("Failed to parse JSON")
                        .and_then(|value| {
                            JupyterMessage::from_value(value)
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
