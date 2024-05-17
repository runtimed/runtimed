use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::media::MimeBundle;

use super::JupyterMessage;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JupyterMessageContent {
    CommClose(CommClose),
    CommInfoReply(CommInfoReply),
    CommInfoRequest(CommInfoRequest),
    CommMsg(CommMsg),
    CommOpen(CommOpen),
    CompleteReply(CompleteReply),
    CompleteRequest(CompleteRequest),
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
    InterruptReply(InterruptReply),
    InterruptRequest(InterruptRequest),
    IsCompleteReply(IsCompleteReply),
    IsCompleteRequest(IsCompleteRequest),
    KernelInfoReply(KernelInfoReply),
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
            JupyterMessageContent::CommClose(_) => "comm_close",
            JupyterMessageContent::CommInfoReply(_) => "comm_info_reply",
            JupyterMessageContent::CommInfoRequest(_) => "comm_info_request",
            JupyterMessageContent::CommMsg(_) => "comm_msg",
            JupyterMessageContent::CommOpen(_) => "comm_open",
            JupyterMessageContent::CompleteReply(_) => "complete_reply",
            JupyterMessageContent::CompleteRequest(_) => "complete_request",
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
            JupyterMessageContent::InterruptReply(__) => "interrupt_reply",
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

            "input_request" => Ok(JupyterMessageContent::InputRequest(serde_json::from_value(
                content,
            )?)),
            "input_reply" => Ok(JupyterMessageContent::InputReply(serde_json::from_value(
                content,
            )?)),

            "is_complete_request" => Ok(JupyterMessageContent::IsCompleteRequest(
                serde_json::from_value(content)?,
            )),
            "is_complete_reply" => Ok(JupyterMessageContent::IsCompleteReply(
                serde_json::from_value(content)?,
            )),

            "kernel_info_request" => Ok(JupyterMessageContent::KernelInfoRequest(
                serde_json::from_value(content)?,
            )),
            "kernel_info_reply" => Ok(JupyterMessageContent::KernelInfoReply(
                serde_json::from_value(content)?,
            )),

            "shutdown_request" => Ok(JupyterMessageContent::ShutdownRequest(
                serde_json::from_value(content)?,
            )),
            "shutdown_reply" => Ok(JupyterMessageContent::ShutdownReply(
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteRequest {
    pub code: String,
    pub silent: bool,
    pub store_history: bool,
    pub user_expressions: Option<HashMap<String, String>>,
    #[serde(default = "default_allow_stdin")]
    pub allow_stdin: bool,
    #[serde(default = "default_stop_on_error")]
    pub stop_on_error: bool,
}

fn default_allow_stdin() -> bool {
    false
}

fn default_stop_on_error() -> bool {
    true
}

pub trait AsChildOf {
    fn as_child_of(self, parent: &JupyterMessage) -> JupyterMessage;
}

macro_rules! impl_as_child_of {
    ($content_type:path, $variant:ident) => {
        impl AsChildOf for $content_type {
            #[doc = concat!("Create a new `JupyterMessage`, assigning the parent for a `", stringify!($content_type), "` message.\n")]
            ///
            /// This method creates a new `JupyterMessage` with the right content, parent header, and zmq identities, making
            /// it suitable for sending over ZeroMQ.
            ///
            /// # Example
            /// ```
            /// use runtimelib::messaging::{JupyterMessage, JupyterMessageContent, AsChildOf};
            ///
            /// let message = connection.recv().await?;
            ///
            #[doc = concat!("let child_message = ", stringify!($content_type), "{\n")]
            ///   // ...
            /// }.as_child_of(&message);
            ///
            /// connection.send(child_message).await?;
            /// ```
            #[must_use]
            fn as_child_of(self, parent: &JupyterMessage) -> JupyterMessage {
                JupyterMessage::new(JupyterMessageContent::$variant(self), Some(parent))
            }
        }

        impl From<$content_type> for JupyterMessage {
            #[doc = concat!("Create a new `JupyterMessage` for a `", stringify!($content_type), "`.\n\n")]
            /// ⚠️ If you use this method, you must set the zmq identities yourself. If you have a message that
            /// "caused" your message to be sent, use that message with `as_child_of` instead.
            #[must_use]
            fn from(content: $content_type) -> Self {
                JupyterMessage::new(JupyterMessageContent::$variant(content), None)
            }
        }
    };
}

impl_as_child_of!(CommClose, CommClose);
impl_as_child_of!(CommInfoReply, CommInfoReply);
impl_as_child_of!(CommInfoRequest, CommInfoRequest);
impl_as_child_of!(CommMsg, CommMsg);
impl_as_child_of!(CommOpen, CommOpen);
impl_as_child_of!(CompleteReply, CompleteReply);
impl_as_child_of!(CompleteRequest, CompleteRequest);
impl_as_child_of!(DisplayData, DisplayData);
impl_as_child_of!(ErrorOutput, ErrorOutput);
impl_as_child_of!(ExecuteInput, ExecuteInput);
impl_as_child_of!(ExecuteReply, ExecuteReply);
impl_as_child_of!(ExecuteRequest, ExecuteRequest);
impl_as_child_of!(ExecuteResult, ExecuteResult);
impl_as_child_of!(HistoryReply, HistoryReply);
impl_as_child_of!(HistoryRequest, HistoryRequest);
impl_as_child_of!(InputReply, InputReply);
impl_as_child_of!(InputRequest, InputRequest);
impl_as_child_of!(IsCompleteReply, IsCompleteReply);
impl_as_child_of!(IsCompleteRequest, IsCompleteRequest);
impl_as_child_of!(KernelInfoReply, KernelInfoReply);
impl_as_child_of!(KernelInfoRequest, KernelInfoRequest);
impl_as_child_of!(ShutdownReply, ShutdownReply);
impl_as_child_of!(ShutdownRequest, ShutdownRequest);
impl_as_child_of!(Status, Status);
impl_as_child_of!(StreamContent, StreamContent);
impl_as_child_of!(UpdateDisplayData, UpdateDisplayData);

/// Unknown message types are a workaround for generically unknown messages.
///
/// ```rust
/// use runtimelib::messaging::{JupyterMessage, JupyterMessageContent, UnknownMessage};
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

impl UnknownMessage {
    pub fn reply(&self, content: serde_json::Value) -> JupyterMessageContent {
        JupyterMessageContent::UnknownMessage(UnknownMessage {
            msg_type: self.msg_type.replace("_request", "_reply"),
            content,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteReply {
    pub status: String,
    pub execution_count: usize,

    pub payload: Option<serde_json::Value>,
    pub user_expressions: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfoRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfoReply {
    #[serde(default = "default_status")]
    pub status: String,
    pub protocol_version: String,
    pub implementation: String,
    pub implementation_version: String,
    pub language_info: LanguageInfo,
    pub banner: String,
    pub help_links: Vec<HelpLink>,
    #[serde(default = "default_debugger")]
    pub debugger: bool,
}

fn default_debugger() -> bool {
    false
}

fn default_status() -> String {
    "ok".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub mimetype: String,
    pub file_extension: String,
    pub pygments_lexer: String,
    pub codemirror_mode: Value,
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
/// ```rust,ignore
/// use runtimelib::messaging::{ExecuteReqeuest};
/// // From the UI
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
/// As a side effect of execution, the kenel can send `'stream'` messages to the UI/client.
/// These are from using `print()`, `console.log()`, or similar. Anything on STDOUT or STDERR.
///
/// ```rust,ignore
/// let execute_request = shell.read().await?; // Should be the execute_request
///
/// let message = StreamContent(Stdio::Stdout).child_of(execute_request);
/// iopub.send(message).await?;
///
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamContent {
    pub name: Stdio,
    pub text: String,
}

/// Optional metadata for a display data to allow for updating an output.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transient {
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
/// use runtimelib::messaging::{ExecuteReqeuest};
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
/// As a side effect of execution, the kenel can send `'display_data'` messages to the UI/client.
///
/// ```rust,ignore
/// use runtimelib::media::{MimeBundle, MimeType, DisplayData};
///
/// let execute_request = shell.read().await?; // Should be the execute_request
///
/// let raw = r#"{
///     "text/plain": "Hello, world!",
///     "text/html": "<h1>Hello, world!</h1>",
/// }"#;
///
/// let bundle: MimeBundle = serde_json::from_str(raw).unwrap();
///
/// let message = DisplayData{
///    data: bundle,
///    metadata: Default::default(),
///    transient: None,
/// }.child_of(execute_request);
/// iopub.send(message).await?;
///
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisplayData {
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Option<Transient>,
}

/// A `'update_display_data'` message on the `'iopub'` channel.
/// See [Update Display Data](https://jupyter-client.readthedocs.io/en/latest/messaging.html#update-display-data).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateDisplayData {
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Transient,
}

/// An `'execute_input'` message on the `'iopub'` channel.
/// See [Execute Input](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute-input).
///
/// To let all frontends know what code is being executed at any given time, these messages contain a re-broadcast of the code portion of an execute_request, along with the execution_count.
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteInput {
    pub code: String,
    pub execution_count: usize,
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
    pub execution_count: usize,
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Option<Transient>,
}

/// An `'error'` message on the `'iopub'` channel.
/// See [Error](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execution-errors).
///
/// These are errors that occur during execution from user code. Syntax errors, runtime errors, etc.
///
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub comm_id: String,
    pub target_name: String,
    pub data: HashMap<String, Value>,
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
    pub comm_id: String,
    pub data: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommInfoRequest {
    pub target_name: String,
}

#[derive(Eq, Hash, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct CommId(String);

impl From<CommId> for String {
    fn from(comm_id: CommId) -> Self {
        comm_id.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommInfo {
    pub target_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommInfoReply {
    pub status: String,
    pub comms: HashMap<CommId, CommInfo>,
}

/// A `comm_close` message on the `'iopub'` channel.
///
/// Since comms live on both sides, when a comm is destroyed the other side must
/// be notified. This is done with a comm_close message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommClose {
    pub comm_id: String,
    pub data: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShutdownRequest {
    pub restart: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptReply {
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShutdownReply {
    pub restart: bool,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InputRequest {
    pub prompt: String,
    pub password: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InputReply {
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompleteRequest {
    pub code: String,
    pub cursor_pos: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompleteReply {
    pub matches: Vec<String>,
    pub cursor_start: usize,
    pub cursor_end: usize,
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IsCompleteReply {
    pub status: String,
    pub indent: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryRequest {
    pub output: bool,
    pub raw: bool,
    pub hist_access_type: String, // This could/should be an enum, which affects the fields below
    pub session: Option<usize>,
    pub start: Option<usize>,
    pub stop: Option<usize>,
    pub n: Option<usize>,
    pub pattern: Option<String>,
    pub unique: Option<bool>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryReply {
    pub history: Vec<HistoryEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IsCompleteRequest {
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub execution_state: String,
}

#[cfg(test)]
mod test {
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
        let request_value = serde_json::to_value(&request).unwrap();

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
}
