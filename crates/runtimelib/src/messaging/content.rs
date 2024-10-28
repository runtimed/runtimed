use jupyter_serde::messaging::{JupyterMessageContent, *};

use super::JupyterMessage;

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

impl<T: IntoJupyterMessage> From<T> for JupyterMessage {
    fn from(content: T) -> Self {
        content.into_jupyter_message()
    }
}

impl From<JupyterMessageContent> for JupyterMessage {
    fn from(content: JupyterMessageContent) -> Self {
        JupyterMessage::new(content, None)
    }
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

impl IntoJupyterMessage for KernelInfoReply {
    fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage {
        JupyterMessage::new(
            JupyterMessageContent::KernelInfoReply(Box::new(self.clone())),
            Some(parent),
        )
    }

    fn into_jupyter_message(self) -> JupyterMessage {
        JupyterMessage::new(JupyterMessageContent::KernelInfoReply(Box::new(self)), None)
    }
}
