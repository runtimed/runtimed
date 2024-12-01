//! Defines the core message types and structures for the Jupyter messaging protocol.
//!
//! This module provides implementations for all message types specified in the
//! [Jupyter Client documentation](https://jupyter-client.readthedocs.io/en/latest/messaging.html),
//! including execute requests/replies, completion, inspection, and more.
//!
//! The main types in this module are:
//!
//! - [`JupyterMessage`]: The top-level message structure.
//! - [`JupyterMessageContent`]: An enum representing all possible message content types.
//! - Various request and reply structures for specific message types (e.g., [`ExecuteRequest`], [`KernelInfoReply`]).
//!
//! # Examples
//!
//! Creating an execute request message:
//!
//! ```rust
//! use jupyter_protocol::messaging::{JupyterMessage, ExecuteRequest};
//!
//! let execute_request = ExecuteRequest::new("print('Hello, world!')".to_string());
//! let message: JupyterMessage = execute_request.into();
//! ```
//!
//! Handling a received message:
//!
//! ```rust
//! use jupyter_protocol::messaging::{JupyterMessage, JupyterMessageContent};
//!
//! fn handle_message(msg: JupyterMessage) {
//!     match msg.content {
//!         JupyterMessageContent::ExecuteRequest(req) => {
//!             println!("Received execute request with code: {}", req.code);
//!         },
//!         JupyterMessageContent::KernelInfoRequest(_) => {
//!             println!("Received kernel info request");
//!         },
//!         _ => println!("Received other message type"),
//!     }
//! }
//! ```
use crate::time;

pub use crate::{
    media::{Media, MediaType},
    ExecutionCount,
};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Shell,
    Control,
    Stdin,
    IOPub,
    Heartbeat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UnknownJupyterMessage {
    pub header: Header,
    pub parent_header: Option<Header>,
    pub metadata: Value,
    pub content: Value,
    #[serde(skip_serializing, skip_deserializing)]
    pub buffers: Vec<Bytes>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub msg_id: String,
    pub username: String,
    pub session: String,
    pub date: DateTime<Utc>,
    pub msg_type: String,
    pub version: String,
}

/// Serializes the `parent_header` of a `JupyterMessage`.
///
/// Treats `None` as an empty object to conform to Jupyter's messaging guidelines:
///
/// > If there is no parent, an empty dict should be used.
/// >
/// > — https://jupyter-client.readthedocs.io/en/latest/messaging.html#parent-header
fn serialize_parent_header<S>(
    parent_header: &Option<Header>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match parent_header {
        Some(parent_header) => parent_header.serialize(serializer),
        None => serde_json::Map::new().serialize(serializer),
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct JupyterMessage {
    #[serde(skip_serializing, skip_deserializing)]
    pub zmq_identities: Vec<Bytes>,
    pub header: Header,
    #[serde(serialize_with = "serialize_parent_header")]
    pub parent_header: Option<Header>,
    pub metadata: Value,
    pub content: JupyterMessageContent,
    #[serde(skip_serializing, skip_deserializing)]
    pub buffers: Vec<Bytes>,
    pub channel: Option<Channel>,
}

impl JupyterMessage {
    pub fn new(
        content: impl Into<JupyterMessageContent>,
        parent: Option<&JupyterMessage>,
    ) -> JupyterMessage {
        // Normally a session ID is per client. A higher level wrapper on this API
        // should probably create messages based on a `Session` struct that is stateful.
        // For now, a user can create a message and then set the session ID directly.
        let session = match parent {
            Some(parent) => parent.header.session.clone(),
            None => Uuid::new_v4().to_string(),
        };

        let content = content.into();

        let header = Header {
            msg_id: Uuid::new_v4().to_string(),
            username: "runtimelib".to_string(),
            session,
            date: time::utc_now(),
            msg_type: content.message_type().to_owned(),
            version: "5.3".to_string(),
        };

        JupyterMessage {
            zmq_identities: parent.map_or(Vec::new(), |parent| parent.zmq_identities.clone()),
            header,
            parent_header: parent.map(|parent| parent.header.clone()),
            metadata: json!({}),
            content,
            buffers: Vec::new(),
            channel: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_buffers(mut self, buffers: Vec<Bytes>) -> Self {
        self.buffers = buffers;
        self
    }

    pub fn with_parent(mut self, parent: &JupyterMessage) -> Self {
        self.header.session.clone_from(&parent.header.session);
        self.parent_header = Some(parent.header.clone());
        self.zmq_identities.clone_from(&parent.zmq_identities);
        self
    }

    pub fn with_zmq_identities(mut self, zmq_identities: Vec<Bytes>) -> Self {
        self.zmq_identities = zmq_identities;
        self
    }

    pub fn with_session(mut self, session: &str) -> Self {
        self.header.session = session.to_string();
        self
    }

    pub fn message_type(&self) -> &str {
        self.content.message_type()
    }

    pub fn from_value(message: Value) -> Result<JupyterMessage, anyhow::Error> {
        let message = serde_json::from_value::<UnknownJupyterMessage>(message)?;

        let content =
            JupyterMessageContent::from_type_and_content(&message.header.msg_type, message.content);

        let content = match content {
            Ok(content) => content,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Error deserializing content for msg_type `{}`: {}",
                    &message.header.msg_type,
                    err
                ));
            }
        };

        let message = JupyterMessage {
            zmq_identities: Vec::new(),
            header: message.header,
            parent_header: message.parent_header,
            metadata: message.metadata,
            content,
            buffers: message.buffers,
            channel: None,
        };

        Ok(message)
    }
}

impl fmt::Debug for JupyterMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\nHeader: {}",
            serde_json::to_string_pretty(&self.header).unwrap()
        )?;
        writeln!(
            f,
            "Parent header: {}",
            if let Some(parent_header) = self.parent_header.as_ref() {
                serde_json::to_string_pretty(parent_header).unwrap()
            } else {
                serde_json::to_string_pretty(&serde_json::Map::new()).unwrap()
            }
        )?;
        writeln!(
            f,
            "Metadata: {}",
            serde_json::to_string_pretty(&self.metadata).unwrap()
        )?;
        writeln!(
            f,
            "Content: {}\n",
            serde_json::to_string_pretty(&self.content).unwrap()
        )?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JupyterMessageContent {
    ClearOutput(ClearOutput),
    CommClose(CommClose),
    CommInfoReply(CommInfoReply),
    CommInfoRequest(CommInfoRequest),
    CommMsg(CommMsg),
    CommOpen(CommOpen),
    CompleteReply(CompleteReply),
    CompleteRequest(CompleteRequest),
    DebugReply(DebugReply),
    DebugRequest(DebugRequest),
    DisplayData(DisplayData),
    ErrorOutput(ErrorOutput),
    ExecuteInput(ExecuteInput),
    ExecuteReply(ExecuteReply),
    ExecuteRequest(ExecuteRequest),
    ExecuteResult(ExecuteResult),
    HistoryReply(HistoryReply),
    HistoryRequest(HistoryRequest),
    InputReply(InputReply),
    InputRequest(InputRequest),
    InspectReply(InspectReply),
    InspectRequest(InspectRequest),
    InterruptReply(InterruptReply),
    InterruptRequest(InterruptRequest),
    IsCompleteReply(IsCompleteReply),
    IsCompleteRequest(IsCompleteRequest),
    // This field is much larger than the most frequent ones
    // so we box it.
    KernelInfoReply(Box<KernelInfoReply>),
    KernelInfoRequest(KernelInfoRequest),
    ShutdownReply(ShutdownReply),
    ShutdownRequest(ShutdownRequest),
    Status(Status),
    StreamContent(StreamContent),
    UnknownMessage(UnknownMessage),
    UpdateDisplayData(UpdateDisplayData),
}

impl JupyterMessageContent {
    pub fn message_type(&self) -> &str {
        match self {
            JupyterMessageContent::ClearOutput(_) => "clear_output",
            JupyterMessageContent::CommClose(_) => "comm_close",
            JupyterMessageContent::CommInfoReply(_) => "comm_info_reply",
            JupyterMessageContent::CommInfoRequest(_) => "comm_info_request",
            JupyterMessageContent::CommMsg(_) => "comm_msg",
            JupyterMessageContent::CommOpen(_) => "comm_open",
            JupyterMessageContent::CompleteReply(_) => "complete_reply",
            JupyterMessageContent::CompleteRequest(_) => "complete_request",
            JupyterMessageContent::DebugReply(_) => "debug_reply",
            JupyterMessageContent::DebugRequest(_) => "debug_request",
            JupyterMessageContent::DisplayData(_) => "display_data",
            JupyterMessageContent::ErrorOutput(_) => "error",
            JupyterMessageContent::ExecuteInput(_) => "execute_input",
            JupyterMessageContent::ExecuteReply(_) => "execute_reply",
            JupyterMessageContent::ExecuteRequest(_) => "execute_request",
            JupyterMessageContent::ExecuteResult(_) => "execute_result",
            JupyterMessageContent::HistoryReply(_) => "history_reply",
            JupyterMessageContent::HistoryRequest(_) => "history_request",
            JupyterMessageContent::InputReply(_) => "input_reply",
            JupyterMessageContent::InputRequest(_) => "input_request",
            JupyterMessageContent::InspectReply(_) => "inspect_reply",
            JupyterMessageContent::InspectRequest(_) => "inspect_request",
            JupyterMessageContent::InterruptReply(_) => "interrupt_reply",
            JupyterMessageContent::InterruptRequest(_) => "interrupt_request",
            JupyterMessageContent::IsCompleteReply(_) => "is_complete_reply",
            JupyterMessageContent::IsCompleteRequest(_) => "is_complete_request",
            JupyterMessageContent::KernelInfoReply(_) => "kernel_info_reply",
            JupyterMessageContent::KernelInfoRequest(_) => "kernel_info_request",
            JupyterMessageContent::ShutdownReply(_) => "shutdown_reply",
            JupyterMessageContent::ShutdownRequest(_) => "shutdown_request",
            JupyterMessageContent::Status(_) => "status",
            JupyterMessageContent::StreamContent(_) => "stream",
            JupyterMessageContent::UnknownMessage(unk) => unk.msg_type.as_str(),
            JupyterMessageContent::UpdateDisplayData(_) => "update_display_data",
        }
    }

    pub fn from_type_and_content(msg_type: &str, content: Value) -> serde_json::Result<Self> {
        match msg_type {
            "clear_output" => Ok(JupyterMessageContent::ClearOutput(serde_json::from_value(
                content,
            )?)),

            "comm_close" => Ok(JupyterMessageContent::CommClose(serde_json::from_value(
                content,
            )?)),

            "comm_info_reply" => Ok(JupyterMessageContent::CommInfoReply(
                serde_json::from_value(content)?,
            )),
            "comm_info_request" => Ok(JupyterMessageContent::CommInfoRequest(
                serde_json::from_value(content)?,
            )),

            "comm_msg" => Ok(JupyterMessageContent::CommMsg(serde_json::from_value(
                content,
            )?)),
            "comm_open" => Ok(JupyterMessageContent::CommOpen(serde_json::from_value(
                content,
            )?)),

            "complete_reply" => Ok(JupyterMessageContent::CompleteReply(
                serde_json::from_value(content)?,
            )),
            "complete_request" => Ok(JupyterMessageContent::CompleteRequest(
                serde_json::from_value(content)?,
            )),

            "debug_reply" => Ok(JupyterMessageContent::DebugReply(serde_json::from_value(
                content,
            )?)),
            "debug_request" => Ok(JupyterMessageContent::DebugRequest(serde_json::from_value(
                content,
            )?)),

            "display_data" => Ok(JupyterMessageContent::DisplayData(serde_json::from_value(
                content,
            )?)),

            "error" => Ok(JupyterMessageContent::ErrorOutput(serde_json::from_value(
                content,
            )?)),

            "execute_input" => Ok(JupyterMessageContent::ExecuteInput(serde_json::from_value(
                content,
            )?)),

            "execute_reply" => Ok(JupyterMessageContent::ExecuteReply(serde_json::from_value(
                content,
            )?)),
            "execute_request" => Ok(JupyterMessageContent::ExecuteRequest(
                serde_json::from_value(content)?,
            )),

            "execute_result" => Ok(JupyterMessageContent::ExecuteResult(
                serde_json::from_value(content)?,
            )),

            "history_reply" => Ok(JupyterMessageContent::HistoryReply(serde_json::from_value(
                content,
            )?)),
            "history_request" => Ok(JupyterMessageContent::HistoryRequest(
                serde_json::from_value(content)?,
            )),

            "input_reply" => Ok(JupyterMessageContent::InputReply(serde_json::from_value(
                content,
            )?)),
            "input_request" => Ok(JupyterMessageContent::InputRequest(serde_json::from_value(
                content,
            )?)),

            "inspect_reply" => Ok(JupyterMessageContent::InspectReply(serde_json::from_value(
                content,
            )?)),
            "inspect_request" => Ok(JupyterMessageContent::InspectRequest(
                serde_json::from_value(content)?,
            )),

            "interrupt_reply" => Ok(JupyterMessageContent::InterruptReply(
                serde_json::from_value(content)?,
            )),
            "interrupt_request" => Ok(JupyterMessageContent::InterruptRequest(
                serde_json::from_value(content)?,
            )),

            "is_complete_reply" => Ok(JupyterMessageContent::IsCompleteReply(
                serde_json::from_value(content)?,
            )),
            "is_complete_request" => Ok(JupyterMessageContent::IsCompleteRequest(
                serde_json::from_value(content)?,
            )),

            "kernel_info_reply" => Ok(JupyterMessageContent::KernelInfoReply(
                serde_json::from_value(content)?,
            )),
            "kernel_info_request" => Ok(JupyterMessageContent::KernelInfoRequest(
                serde_json::from_value(content)?,
            )),

            "shutdown_reply" => Ok(JupyterMessageContent::ShutdownReply(
                serde_json::from_value(content)?,
            )),
            "shutdown_request" => Ok(JupyterMessageContent::ShutdownRequest(
                serde_json::from_value(content)?,
            )),

            "status" => Ok(JupyterMessageContent::Status(serde_json::from_value(
                content,
            )?)),

            "stream" => Ok(JupyterMessageContent::StreamContent(
                serde_json::from_value(content)?,
            )),

            "update_display_data" => Ok(JupyterMessageContent::UpdateDisplayData(
                serde_json::from_value(content)?,
            )),

            _ => Ok(JupyterMessageContent::UnknownMessage(UnknownMessage {
                msg_type: msg_type.to_string(),
                content,
            })),
        }
    }
}

macro_rules! impl_message_traits {
    ($($name:ident),*) => {
        $(
            impl $name {
                #[doc = concat!("Create a new `JupyterMessage`, assigning the parent for a `", stringify!($name), "` message.\n")]
                ///
                /// This method creates a new `JupyterMessage` with the right content, parent header, and zmq identities, making
                /// it suitable for sending over ZeroMQ.
                ///
                /// # Example
                /// ```rust
                /// use jupyter_protocol::messaging::{JupyterMessage, JupyterMessageContent};
                #[doc = concat!("use jupyter_protocol::", stringify!($name), ";\n")]
                ///
                /// let parent_message = JupyterMessage::new(jupyter_protocol::UnknownMessage {
                ///   msg_type: "example".to_string(),
                ///   content: serde_json::json!({ "key": "value" }),
                /// }, None);
                ///
                #[doc = concat!("let child_message = ", stringify!($name), "{\n")]
                ///   ..Default::default()
                /// }.as_child_of(&parent_message);
                ///
                /// // Next you would send the `child_message` over the connection
                ///
                /// ```
                #[must_use]
                pub fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage {
                    JupyterMessage::new(self.clone(), Some(parent))
                }
            }

            impl From<$name> for JupyterMessage {
                #[doc = concat!("Create a new `JupyterMessage` for a `", stringify!($name), "`.\n\n")]
                /// ⚠️ If you use this method with `runtimelib`, you must set the zmq identities yourself. If you
                /// have a message that "caused" your message to be sent, use that message with `as_child_of` instead.
                #[must_use]
                fn from(content: $name) -> Self {
                    JupyterMessage::new(content, None)
                }
            }

            impl From<$name> for JupyterMessageContent {
                #[doc = concat!("Create a new `JupyterMessageContent` for a `", stringify!($name), "`.\n\n")]
                #[must_use]
                fn from(content: $name) -> Self {
                    JupyterMessageContent::$name(content)
                }
            }
        )*
    };
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
    // HistoryRequest, // special case due to enum entry
    InputReply,
    InputRequest,
    InspectReply,
    InspectRequest,
    InterruptReply,
    InterruptRequest,
    IsCompleteReply,
    IsCompleteRequest,
    // KernelInfoReply, // special case due to boxing
    KernelInfoRequest,
    ShutdownReply,
    ShutdownRequest,
    Status,
    StreamContent,
    UpdateDisplayData,
    UnknownMessage
);

// KernelInfoReply is a special case due to the Boxing requirement
impl KernelInfoReply {
    pub fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage {
        JupyterMessage::new(
            JupyterMessageContent::KernelInfoReply(Box::new(self.clone())),
            Some(parent),
        )
    }
}

impl From<KernelInfoReply> for JupyterMessage {
    fn from(content: KernelInfoReply) -> Self {
        JupyterMessage::new(
            JupyterMessageContent::KernelInfoReply(Box::new(content)),
            None,
        )
    }
}

impl From<KernelInfoReply> for JupyterMessageContent {
    fn from(content: KernelInfoReply) -> Self {
        JupyterMessageContent::KernelInfoReply(Box::new(content))
    }
}

impl HistoryRequest {
    /// Create a new `JupyterMessage`, assigning the parent for a `HistoryRequest` message.
    ///
    /// This method creates a new `JupyterMessage` with the right content, parent header, and zmq identities, making
    /// it suitable for sending over ZeroMQ.
    ///
    /// # Example
    /// ```rust
    /// use jupyter_protocol::messaging::{JupyterMessage, JupyterMessageContent};
    /// use jupyter_protocol::HistoryRequest;
    ///
    /// let parent_message = JupyterMessage::new(jupyter_protocol::UnknownMessage {
    ///   msg_type: "example".to_string(),
    ///   content: serde_json::json!({ "key": "value" }),
    /// }, None);
    ///
    /// let child_message = HistoryRequest::Range {
    ///     session: None,
    ///     start: 0,
    ///     stop: 10,
    ///     output: false,
    ///     raw: false,
    /// }.as_child_of(&parent_message);
    ///
    /// // Next you would send the `child_message` over the connection
    /// ```
    #[must_use]
    pub fn as_child_of(&self, parent: &JupyterMessage) -> JupyterMessage {
        JupyterMessage::new(self.clone(), Some(parent))
    }
}

impl From<HistoryRequest> for JupyterMessage {
    /// Create a new `JupyterMessage` for a `HistoryRequest`.
    /// ⚠️ If you use this method with `runtimelib`, you must set the zmq identities yourself. If you
    /// have a message that "caused" your message to be sent, use that message with `as_child_of` instead.
    #[must_use]
    fn from(content: HistoryRequest) -> Self {
        JupyterMessage::new(content, None)
    }
}

impl From<HistoryRequest> for JupyterMessageContent {
    /// Create a new `JupyterMessageContent` for a `HistoryRequest`.
    #[must_use]
    fn from(content: HistoryRequest) -> Self {
        JupyterMessageContent::HistoryRequest(content)
    }
}

/// Unknown message types are a workaround for generically unknown messages.
///
/// ```rust
/// use jupyter_protocol::messaging::{JupyterMessage, JupyterMessageContent, UnknownMessage};
/// use serde_json::json;
///
/// let msg = UnknownMessage {
///     msg_type: "example_request".to_string(),
///     content: json!({ "key": "value" }),
/// };
///
/// let reply_msg = msg.reply(json!({ "status": "ok" }));
/// ```
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnknownMessage {
    #[serde(skip_serializing, skip_deserializing)]
    pub msg_type: String,
    #[serde(flatten)]
    pub content: Value,
}
impl Default for UnknownMessage {
    fn default() -> Self {
        Self {
            msg_type: "unknown".to_string(),
            content: Value::Null,
        }
    }
}

impl UnknownMessage {
    // Create a reply message for an unknown message, assuming `content` is known.
    // Useful for when runtimelib does not support the message type.
    // Send a PR to add support for the message type!
    pub fn reply(&self, content: serde_json::Value) -> JupyterMessageContent {
        JupyterMessageContent::UnknownMessage(UnknownMessage {
            msg_type: self.msg_type.replace("_request", "_reply"),
            content,
        })
    }
}

/// All reply messages have a `status` field.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReplyStatus {
    #[default]
    Ok,
    Error,
    Aborted,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReplyError {
    pub ename: String,
    pub evalue: String,
    pub traceback: Vec<String>,
}

/// Clear output of a single cell / output area.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ClearOutput {
    /// Wait to clear the output until new output is available.  Clears the

    /// existing output immediately before the new output is displayed.
    /// Useful for creating simple animations with minimal flickering.
    pub wait: bool,
}

/// A request for code execution.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteRequest {
    pub code: String,
    pub silent: bool,
    pub store_history: bool,
    #[serde(serialize_with = "serialize_user_expressions")]
    pub user_expressions: Option<HashMap<String, String>>,
    #[serde(default = "default_allow_stdin")]
    pub allow_stdin: bool,
    #[serde(default = "default_stop_on_error")]
    pub stop_on_error: bool,
}

/// Serializes the `user_expressions`.
///
/// Treats `None` as an empty object to conform to Jupyter's messaging guidelines.
fn serialize_user_expressions<S>(
    user_expressions: &Option<HashMap<String, String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match user_expressions {
        Some(user_expressions) => user_expressions.serialize(serializer),
        None => serde_json::Map::new().serialize(serializer),
    }
}

fn default_allow_stdin() -> bool {
    false
}

fn default_stop_on_error() -> bool {
    true
}

impl ExecuteRequest {
    pub fn new(code: String) -> Self {
        Self {
            code,
            ..Default::default()
        }
    }
}

impl Default for ExecuteRequest {
    fn default() -> Self {
        Self {
            code: "".to_string(),
            silent: false,
            store_history: true,
            user_expressions: None,
            allow_stdin: false,
            stop_on_error: true,
        }
    }
}

/// A reply to an execute request.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#execution-results>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteReply {
    pub status: ReplyStatus,
    pub execution_count: ExecutionCount,

    #[serde(default)]
    pub payload: Vec<Payload>,
    pub user_expressions: Option<HashMap<String, String>>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for ExecuteReply {
    fn default() -> Self {
        Self {
            status: ReplyStatus::Ok,
            execution_count: ExecutionCount::new(0),
            payload: Vec::new(),
            user_expressions: None,
            error: None,
        }
    }
}

/// Payloads are a way to trigger frontend actions from the kernel.
/// They are stated as deprecated, however they are in regular use via `?` in IPython
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#payloads-deprecated>
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "source")]
pub enum Payload {
    Page {
        data: Media,
        start: usize,
    },
    SetNextInput {
        text: String,
        replace: bool,
    },
    EditMagic {
        filename: String,
        line_number: usize,
    },
    AskExit {
        // sic
        keepkernel: bool,
    },
}

/// A request for information about the kernel.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-info>
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct KernelInfoRequest {}

/// A reply containing information about the kernel.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-info>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfoReply {
    pub status: ReplyStatus,
    pub protocol_version: String,
    pub implementation: String,
    pub implementation_version: String,
    pub language_info: LanguageInfo,
    pub banner: String,
    pub help_links: Vec<HelpLink>,
    #[serde(default = "default_debugger")]
    pub debugger: bool,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}

fn default_debugger() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CodeMirrorMode {
    Simple(String),
    CustomMode { name: String, version: usize },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CodeMirrorModeObject {
    pub name: String,
    pub version: usize,
}

impl CodeMirrorMode {
    pub fn typescript() -> Self {
        Self::Simple("typescript".to_string())
    }

    pub fn python() -> Self {
        Self::Simple("python".to_string())
    }

    pub fn ipython_code_mirror_mode() -> Self {
        Self::CustomMode {
            name: "ipython".to_string(),
            version: 3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub mimetype: String,
    pub file_extension: String,
    pub pygments_lexer: String,
    pub codemirror_mode: CodeMirrorMode,
    pub nbconvert_exporter: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HelpLink {
    pub text: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Stdio {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
}

/// A `'stream'` message on the `'iopub'` channel.
///
/// See [Streams](https://jupyter-client.readthedocs.io/en/latest/messaging.html#streams-stdout-stderr-etc).
///
/// ## Example
/// The UI/client sends an `'execute_request'` message to the kernel.
///
/// ```rust
/// use jupyter_protocol::{ExecuteRequest, JupyterMessage};
/// // The UI/client sends an `'execute_request'` message to the kernel.
///
/// let execute_request = ExecuteRequest {
///     code: "print('Hello, World!')".to_string(),
///     silent: false,
///     store_history: true,
///     user_expressions: None,
///     allow_stdin: false,
///     stop_on_error: true,
/// };
///
/// let incoming_message: JupyterMessage = execute_request.into();
///
/// // ...
///
///
/// // On the kernel side, we receive the `'execute_request'` message.
/// //
/// // As a side effect of execution, the kernel can send `'stream'` messages to the UI/client.
/// // These are from using `print()`, `console.log()`, or similar. Anything on STDOUT or STDERR.
///
/// use jupyter_protocol::{StreamContent, Stdio};
///
/// let message = StreamContent {
///   name: Stdio::Stdout,
///   text: "Hello, World".to_string()
/// // To inform the UI that the kernel is emitting this stdout in response to the execution, we
/// // use `as_child_of` to set the parent header.
/// }.as_child_of(&incoming_message);
///
/// // next, send the `message` back over the iopub connection
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamContent {
    pub name: Stdio,
    pub text: String,
}
impl Default for StreamContent {
    fn default() -> Self {
        Self {
            name: Stdio::Stdout,
            text: String::new(),
        }
    }
}

impl StreamContent {
    pub fn stdout(text: &str) -> Self {
        Self {
            name: Stdio::Stdout,
            text: text.to_string(),
        }
    }

    pub fn stderr(text: &str) -> Self {
        Self {
            name: Stdio::Stderr,
            text: text.to_string(),
        }
    }
}

/// Optional metadata for a display data to allow for updating an output.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Transient {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_id: Option<String>,
}

/// A `'display_data'` message on the `'iopub'` channel.
///
/// See [Display Data](https://jupyter-client.readthedocs.io/en/latest/messaging.html#display-data).
///
/// ## Example
///
/// The UI/client sends an `'execute_request'` message to the kernel.
///
/// ```rust,ignore
/// use jupyter_protocol::messaging::{ExecuteReqeuest};
///
/// let execute_request = ExecuteRequest {
///     code: "print('Hello, World!')".to_string(),
///     silent: false,
///     store_history: true,
///     user_expressions: None,
///     allow_stdin: false,
///     stop_on_error: true,
/// };
/// connection.send(execute_request).await?;
/// ```
///
/// As a side effect of execution, the kernel can send `'display_data'` messages to the UI/client.
///
/// ```rust,ignore
/// use jupyter_protocol::media::{Media, MediaType, DisplayData};
///
/// let execute_request = shell.read().await?;
///
/// let raw = r#"{
///     "text/plain": "Hello, world!",
///     "text/html": "<h1>Hello, world!</h1>",
/// }"#;
///
/// let bundle: Media = serde_json::from_str(raw).unwrap();
///
/// let message = DisplayData{
///    data: bundle,
///    metadata: Default::default(),
///    transient: None,
/// }.as_child_of(execute_request);
/// iopub.send(message).await?;
///
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DisplayData {
    pub data: Media,
    pub metadata: serde_json::Map<String, Value>,
    #[serde(default)]
    pub transient: Transient,
}

impl DisplayData {
    pub fn new(data: Media) -> Self {
        Self {
            data,
            metadata: Default::default(),
            transient: Default::default(),
        }
    }
}

impl From<Vec<MediaType>> for DisplayData {
    fn from(content: Vec<MediaType>) -> Self {
        Self::new(Media::new(content))
    }
}

impl From<MediaType> for DisplayData {
    fn from(content: MediaType) -> Self {
        Self::new(Media::new(vec![content]))
    }
}

/// A `'update_display_data'` message on the `'iopub'` channel.
/// See [Update Display Data](https://jupyter-client.readthedocs.io/en/latest/messaging.html#update-display-data).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateDisplayData {
    pub data: Media,
    pub metadata: serde_json::Map<String, Value>,
    pub transient: Transient,
}

impl UpdateDisplayData {
    pub fn new(data: Media, display_id: &str) -> Self {
        Self {
            data,
            metadata: Default::default(),
            transient: Transient {
                display_id: Some(display_id.to_string()),
            },
        }
    }
}

/// An `'execute_input'` message on the `'iopub'` channel.
/// See [Execute Input](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute-input).
///
/// To let all frontends know what code is being executed at any given time, these messages contain a re-broadcast of the code portion of an execute_request, along with the execution_count.
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteInput {
    pub code: String,
    pub execution_count: ExecutionCount,
}
impl Default for ExecuteInput {
    fn default() -> Self {
        Self {
            code: String::new(),
            execution_count: ExecutionCount::new(0),
        }
    }
}

/// An `'execute_result'` message on the `'iopub'` channel.
/// See [Execute Result](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute-result).
///
/// The is the "result", in the REPL sense from execution. As an example, the following Python code:
///
/// ```python
/// >>> 3 + 4
/// 7
/// ```
///
/// would have an `'execute_result'` message with the following content:
///
/// ```json
/// {
///     "execution_count": 1,
///     "data": {
///         "text/plain": "7"
///     },
///     "metadata": {},
///     "transient": {}
/// }
/// ```
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteResult {
    pub execution_count: ExecutionCount,
    pub data: Media,
    pub metadata: serde_json::Map<String, Value>,
    pub transient: Option<Transient>,
}
impl Default for ExecuteResult {
    fn default() -> Self {
        Self {
            execution_count: ExecutionCount::new(0),
            data: Media::default(),
            metadata: serde_json::Map::new(),
            transient: None,
        }
    }
}

impl ExecuteResult {
    pub fn new(execution_count: ExecutionCount, data: Media) -> Self {
        Self {
            execution_count,
            data,
            metadata: Default::default(),
            transient: None,
        }
    }
}

impl From<(ExecutionCount, Vec<MediaType>)> for ExecuteResult {
    fn from((execution_count, content): (ExecutionCount, Vec<MediaType>)) -> Self {
        Self::new(execution_count, content.into())
    }
}

impl From<(ExecutionCount, MediaType)> for ExecuteResult {
    fn from((execution_count, content): (ExecutionCount, MediaType)) -> Self {
        Self::new(execution_count, content.into())
    }
}

/// An `'error'` message on the `'iopub'` channel.
/// See [Error](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execution-errors).
///
/// These are errors that occur during execution from user code. Syntax errors, runtime errors, etc.
///
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ErrorOutput {
    pub ename: String,
    pub evalue: String,
    pub traceback: Vec<String>,
}

/// A `'comm_open'` message on the `'iopub'` channel.
///
/// See [Comm Open](https://jupyter-client.readthedocs.io/en/latest/messaging.html#opening-a-comm).
///
/// Comm messages are one-way communications to update comm state, used for
/// synchronizing widget state, or simply requesting actions of a comm’s
/// counterpart.
///
/// Opening a Comm produces a `comm_open` message, to be sent to the other side:
///
/// ```json
/// {
///   "comm_id": "u-u-i-d",
///   "target_name": "my_comm",
///   "data": {}
/// }
/// ```
///
/// Every Comm has an ID and a target name. The code handling the message on
/// the receiving side is responsible for maintaining a mapping of target_name
/// keys to constructors. After a `comm_open` message has been sent, there
/// should be a corresponding Comm instance on both sides. The data key is
/// always a object with any extra JSON information used in initialization of
/// the comm.
///
/// If the `target_name` key is not found on the receiving side, then it should
/// immediately reply with a `comm_close` message to avoid an inconsistent state.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommOpen {
    pub comm_id: CommId,
    pub target_name: String,
    pub data: serde_json::Map<String, Value>,
}
impl Default for CommOpen {
    fn default() -> Self {
        Self {
            comm_id: CommId("".to_string()),
            target_name: String::new(),
            data: serde_json::Map::new(),
        }
    }
}

/// A `comm_msg` message on the `'iopub'` channel.
///
/// Comm messages are one-way communications to update comm state, used for
/// synchronizing widget state, or simply requesting actions of a comm’s
/// counterpart.
///
/// Essentially, each comm pair defines their own message specification
/// implemented inside the data object.
///
/// There are no expected replies.
///
/// ```json
/// {
///   "comm_id": "u-u-i-d",
///   "data": {}
/// }
/// ```
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommMsg {
    pub comm_id: CommId,
    pub data: serde_json::Map<String, Value>,
}
impl Default for CommMsg {
    fn default() -> Self {
        Self {
            comm_id: CommId("".to_string()),
            data: serde_json::Map::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CommInfoRequest {
    pub target_name: String,
}

#[derive(Eq, Hash, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct CommId(pub String);

impl From<CommId> for String {
    fn from(comm_id: CommId) -> Self {
        comm_id.0
    }
}

impl From<String> for CommId {
    fn from(comm_id: String) -> Self {
        Self(comm_id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommInfo {
    pub target_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommInfoReply {
    pub status: ReplyStatus,
    pub comms: HashMap<CommId, CommInfo>,
    // pub comms: HashMap<CommId, CommInfo>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for CommInfoReply {
    fn default() -> Self {
        Self {
            status: ReplyStatus::Ok,
            comms: HashMap::new(),
            error: None,
        }
    }
}

/// A `comm_close` message on the `'iopub'` channel.
///
/// Since comms live on both sides, when a comm is destroyed the other side must
/// be notified. This is done with a comm_close message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommClose {
    pub comm_id: CommId,
    pub data: serde_json::Map<String, Value>,
}
impl Default for CommClose {
    fn default() -> Self {
        Self {
            comm_id: CommId("".to_string()),
            data: serde_json::Map::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// Request to shut down the kernel.
///
/// Upon receiving this message, the kernel will send a reply and then shut itself down.
/// If `restart` is True, the kernel will restart itself after shutting down.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-shutdown>
pub struct ShutdownRequest {
    pub restart: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// Request to interrupt the kernel.
///
/// This message is used when the kernel's `interrupt_mode` is set to "message"
/// in its kernelspec. It allows the kernel to be interrupted via a message
/// instead of an operating system signal.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-interrupt>
pub struct InterruptRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Reply to an interrupt request.
///
/// This message is sent by the kernel in response to an `InterruptRequest`.
/// It indicates whether the interrupt was successful.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-interrupt>
pub struct InterruptReply {
    pub status: ReplyStatus,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}

impl Default for InterruptReply {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptReply {
    pub fn new() -> Self {
        Self {
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Reply to a shutdown request.
///
/// This message is sent by the kernel in response to a `ShutdownRequest`.
/// It confirms that the kernel is shutting down.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-shutdown>
pub struct ShutdownReply {
    pub restart: bool,
    pub status: ReplyStatus,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for ShutdownReply {
    fn default() -> Self {
        Self {
            restart: false,
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Request for input from the frontend.
///
/// This message is sent by the kernel when it needs to prompt the user for input.
/// It's typically used to implement functions like Python's `input()` or R's `readline()`.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#messages-on-the-stdin-router-dealer-channel>
pub struct InputRequest {
    pub prompt: String,
    pub password: bool,
}
impl Default for InputRequest {
    fn default() -> Self {
        Self {
            prompt: "> ".to_string(),
            password: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Reply to an input request.
///
/// This message is sent by the frontend in response to an `InputRequest`.
/// It contains the user's input.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#messages-on-the-stdin-router-dealer-channel>
pub struct InputReply {
    pub value: String,

    pub status: ReplyStatus,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for InputReply {
    fn default() -> Self {
        Self {
            value: String::new(),
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

/// A `'inspect_request'` message on the `'shell'` channel.
///
/// Code can be inspected to show useful information to the user.
/// It is up to the Kernel to decide what information should be displayed, and its formatting.
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InspectRequest {
    /// The code context in which introspection is requested
    /// this may be up to an entire multiline cell.
    pub code: String,
    /// The cursor position within 'code' (in unicode characters) where inspection is requested
    pub cursor_pos: usize,
    /// The level of detail desired.  In IPython, the default (0) is equivalent to typing
    /// 'x?' at the prompt, 1 is equivalent to 'x??'.
    /// The difference is up to kernels, but in IPython level 1 includes the source code
    /// if available.
    pub detail_level: Option<usize>,
}
impl Default for InspectRequest {
    fn default() -> Self {
        Self {
            code: String::new(),
            cursor_pos: 0,
            detail_level: Some(0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InspectReply {
    pub found: bool,
    pub data: Media,
    pub metadata: serde_json::Map<String, Value>,

    pub status: ReplyStatus,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for InspectReply {
    fn default() -> Self {
        Self {
            found: false,
            data: Media::default(),
            metadata: serde_json::Map::new(),
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

/// A request for code completion suggestions.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#completion>
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CompleteRequest {
    pub code: String,
    pub cursor_pos: usize,
}

/// A reply containing code completion suggestions.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#completion>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompleteReply {
    pub matches: Vec<String>,
    pub cursor_start: usize,
    pub cursor_end: usize,
    pub metadata: serde_json::Map<String, Value>,

    pub status: ReplyStatus,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for CompleteReply {
    fn default() -> Self {
        Self {
            matches: Vec::new(),
            cursor_start: 0,
            cursor_end: 0,
            metadata: serde_json::Map::new(),
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DebugRequest {
    #[serde(flatten)]
    pub content: Value,
}
impl Default for DebugRequest {
    fn default() -> Self {
        Self {
            content: Value::Null,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DebugReply {
    #[serde(flatten)]
    pub content: Value,
}
impl Default for DebugReply {
    fn default() -> Self {
        Self {
            content: Value::Null,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IsCompleteReplyStatus {
    /// The code is incomplete, and the frontend should prompt the user for more
    /// input.
    Incomplete,
    /// The code is ready to be executed.
    Complete,
    /// The code is invalid, yet can be sent for execution to see a syntax error.
    Invalid,
    /// The kernel is unable to determine status. The frontend should also
    /// handle the kernel not replying promptly. It may default to sending the
    /// code for execution, or it may implement simple fallback heuristics for
    /// whether to execute the code (e.g. execute after a blank line).
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IsCompleteReply {
    /// Unlike other reply messages, the status is unique to this message, using `IsCompleteReplyStatus`
    /// instead of `ReplyStatus`.
    pub status: IsCompleteReplyStatus,
    /// If status is 'incomplete', indent should contain the characters to use
    /// to indent the next line. This is only a hint: frontends may ignore it
    /// and use their own autoindentation rules. For other statuses, this
    /// field does not exist.
    pub indent: String,
}
impl Default for IsCompleteReply {
    fn default() -> Self {
        Self {
            status: IsCompleteReplyStatus::Unknown,
            indent: String::new(),
        }
    }
}

impl IsCompleteReply {
    pub fn new(status: IsCompleteReplyStatus, indent: String) -> Self {
        Self { status, indent }
    }

    pub fn incomplete(indent: String) -> Self {
        Self::new(IsCompleteReplyStatus::Incomplete, indent)
    }

    pub fn complete() -> Self {
        Self::new(IsCompleteReplyStatus::Complete, String::new())
    }

    pub fn invalid() -> Self {
        Self::new(IsCompleteReplyStatus::Invalid, String::new())
    }

    pub fn unknown() -> Self {
        Self::new(IsCompleteReplyStatus::Unknown, String::new())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "hist_access_type")]
pub enum HistoryRequest {
    #[serde(rename = "range")]
    Range {
        session: Option<i32>,
        start: i32,
        stop: i32,
        output: bool,
        raw: bool,
    },
    #[serde(rename = "tail")]
    Tail { n: i32, output: bool, raw: bool },
    #[serde(rename = "search")]
    Search {
        pattern: String,
        unique: bool,
        output: bool,
        raw: bool,
    },
}
impl Default for HistoryRequest {
    fn default() -> Self {
        Self::Range {
            session: None,
            start: 0,
            stop: 0,
            output: false,
            raw: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum HistoryEntry {
    // When history_request.output is false
    // (session, line_number, input)
    Input(usize, usize, String),
    // When history_request.output is true
    // (session, line_number, (input, output))
    InputOutput(usize, usize, (String, String)),
}

/// A reply containing execution history.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#history>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryReply {
    pub history: Vec<HistoryEntry>,

    pub status: ReplyStatus,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<ReplyError>>,
}
impl Default for HistoryReply {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

impl HistoryReply {
    pub fn new(history: Vec<HistoryEntry>) -> Self {
        Self {
            history,
            status: ReplyStatus::Ok,
            error: None,
        }
    }
}

/// A request to check if the code is complete and ready for execution.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#code-completeness>
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IsCompleteRequest {
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionState {
    Busy,
    Idle,
}

impl ExecutionState {
    pub fn as_str(&self) -> &str {
        match self {
            ExecutionState::Busy => "busy",
            ExecutionState::Idle => "idle",
        }
    }
}

/// A message indicating the current status of the kernel.
///
/// See <https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-status>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub execution_state: ExecutionState,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            execution_state: ExecutionState::Idle,
        }
    }
}

impl Status {
    pub fn busy() -> Self {
        Self {
            execution_state: ExecutionState::Busy,
        }
    }

    pub fn idle() -> Self {
        Self {
            execution_state: ExecutionState::Idle,
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_execute_request_serialize() {
        let request = ExecuteRequest {
            code: "print('Hello, World!')".to_string(),
            silent: false,
            store_history: true,
            user_expressions: Some(HashMap::new()),
            allow_stdin: false,
            stop_on_error: true,
        };
        let request_value = serde_json::to_value(request).unwrap();

        let expected_request_value = serde_json::json!({
            "code": "print('Hello, World!')",
            "silent": false,
            "store_history": true,
            "user_expressions": {},
            "allow_stdin": false,
            "stop_on_error": true
        });

        assert_eq!(request_value, expected_request_value);
    }

    #[test]
    fn test_execute_request_user_expressions_serializes_to_empty_dict() {
        let request = ExecuteRequest {
            code: "print('Hello, World!')".to_string(),
            silent: false,
            store_history: true,
            user_expressions: None,
            allow_stdin: false,
            stop_on_error: true,
        };
        let request_value = serde_json::to_value(request).unwrap();

        let expected_request_value = serde_json::json!({
            "code": "print('Hello, World!')",
            "silent": false,
            "store_history": true,
            "user_expressions": {},
            "allow_stdin": false,
            "stop_on_error": true
        });

        assert_eq!(request_value, expected_request_value);
    }

    #[test]
    fn test_into_various() {
        let kernel_info_request = KernelInfoRequest {};
        let content: JupyterMessageContent = kernel_info_request.clone().into();
        let message: JupyterMessage = content.into();
        assert!(message.parent_header.is_none());
        match message.content {
            JupyterMessageContent::KernelInfoRequest(req) => {
                assert_eq!(req, kernel_info_request);
            }
            _ => panic!("Expected KernelInfoRequest"),
        }

        let kernel_info_request = KernelInfoRequest {};
        let message: JupyterMessage = kernel_info_request.clone().into();
        assert!(message.parent_header.is_none());
        match message.content {
            JupyterMessageContent::KernelInfoRequest(req) => {
                assert_eq!(req, kernel_info_request);
            }
            _ => panic!("Expected KernelInfoRequest"),
        }
    }

    #[test]
    fn test_default() {
        let msg: JupyterMessage = ExecuteRequest {
            code: "import this".to_string(),
            ..Default::default()
        }
        .into();

        assert_eq!(msg.header.msg_type, "execute_request");
        assert_eq!(msg.header.msg_id.len(), 36);

        match msg.content {
            JupyterMessageContent::ExecuteRequest(req) => {
                assert_eq!(req.code, "import this");
                assert!(!req.silent);
                assert!(req.store_history);
                assert_eq!(req.user_expressions, None);
                assert!(!req.allow_stdin);
                assert!(req.stop_on_error);
            }
            _ => panic!("Expected ExecuteRequest"),
        }
    }

    #[test]
    fn test_deserialize_payload() {
        let raw_execute_reply_content = r#"
        {
            "status": "ok",
            "execution_count": 1,
            "payload": [{
                "source": "page",
                "data": {
                    "text/html": "<h1>Hello</h1>",
                    "text/plain": "Hello"
                },
                "start": 0
            }],
            "user_expressions": {}
        }
        "#;

        let execute_reply: ExecuteReply = serde_json::from_str(raw_execute_reply_content).unwrap();

        assert_eq!(execute_reply.status, ReplyStatus::Ok);
        assert_eq!(execute_reply.execution_count, ExecutionCount::new(1));

        let payload = execute_reply.payload.clone();

        assert_eq!(payload.len(), 1);
        let payload = payload.first().unwrap();

        let media = match payload {
            Payload::Page { data, .. } => data,
            _ => panic!("Expected Page payload type"),
        };

        let media = serde_json::to_value(media).unwrap();

        let expected_media = serde_json::json!({
            "text/html": "<h1>Hello</h1>",
            "text/plain": "Hello"
        });

        assert_eq!(media, expected_media);
    }

    #[test]
    pub fn test_display_data_various_data() {
        let display_data = DisplayData {
            data: serde_json::from_value(json!({
                "text/plain": "Hello, World!",
                "text/html": "<h1>Hello, World!</h1>",
                "application/json": {
                    "hello": "world",
                    "foo": "bar",
                    "ok": [1, 2, 3],
                }
            }))
            .unwrap(),
            ..Default::default()
        };

        let display_data_value = serde_json::to_value(display_data).unwrap();

        let expected_display_data_value = serde_json::json!({
            "data": {
                "text/plain": "Hello, World!",
                "text/html": "<h1>Hello, World!</h1>",
                "application/json": {
                    "hello": "world",
                    "foo": "bar",
                    "ok": [1, 2, 3]
                }
            },
            "metadata": {},
            "transient": {}
        });

        assert_eq!(display_data_value, expected_display_data_value);
    }

    use std::mem::size_of;

    macro_rules! size_of_variant {
        ($variant:ty) => {
            let size = size_of::<$variant>();
            println!("The size of {} is: {} bytes", stringify!($variant), size);

            assert!(size <= 96);
        };
    }

    #[test]
    fn test_enum_variant_sizes() {
        size_of_variant!(ClearOutput);
        size_of_variant!(CommClose);
        size_of_variant!(CommInfoReply);
        size_of_variant!(CommInfoRequest);
        size_of_variant!(CommMsg);
        size_of_variant!(CommOpen);
        size_of_variant!(CompleteReply);
        size_of_variant!(CompleteRequest);
        size_of_variant!(DebugReply);
        size_of_variant!(DebugRequest);
        size_of_variant!(DisplayData);
        size_of_variant!(ErrorOutput);
        size_of_variant!(ExecuteInput);
        size_of_variant!(ExecuteReply);
        size_of_variant!(ExecuteRequest);
        size_of_variant!(ExecuteResult);
        size_of_variant!(HistoryReply);
        size_of_variant!(HistoryRequest);
        size_of_variant!(InputReply);
        size_of_variant!(InputRequest);
        size_of_variant!(InspectReply);
        size_of_variant!(InspectRequest);
        size_of_variant!(InterruptReply);
        size_of_variant!(InterruptRequest);
        size_of_variant!(IsCompleteReply);
        size_of_variant!(IsCompleteRequest);
        size_of_variant!(Box<KernelInfoReply>);
        size_of_variant!(KernelInfoRequest);
        size_of_variant!(ShutdownReply);
        size_of_variant!(ShutdownRequest);
        size_of_variant!(Status);
        size_of_variant!(StreamContent);
        size_of_variant!(UnknownMessage);
        size_of_variant!(UpdateDisplayData);
    }

    #[test]
    fn test_jupyter_message_content_enum_size() {
        let size = size_of::<JupyterMessageContent>();
        println!("The size of JupyterMessageContent is: {}", size);
        assert!(size > 0);
        assert!(size <= 96);
    }

    #[test]
    fn test_jupyter_message_parent_header_serializes_to_empty_dict() {
        let request = ExecuteRequest {
            code: "1 + 1".to_string(),
            ..Default::default()
        };
        let message = JupyterMessage::from(request);

        let serialized_message = serde_json::to_value(message).unwrap();

        // Test that the `parent_header` field is an empty object.
        let parent_header = serialized_message.get("parent_header").unwrap();
        assert!(parent_header.is_object());
        assert!(parent_header.as_object().unwrap().is_empty());
    }
}
