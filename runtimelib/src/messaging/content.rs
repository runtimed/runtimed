use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::media::MimeBundle;

use super::JupyterMessage;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JupyterMessageContent {
    ExecuteRequest(ExecuteRequest),
    ExecuteReply(ExecuteReply),
    KernelInfoRequest(KernelInfoRequest),
    KernelInfoReply(KernelInfoReply),
    StreamContent(StreamContent),
    DisplayData(DisplayData),
    UpdateDisplayData(UpdateDisplayData),
    ExecuteInput(ExecuteInput),
    ExecuteResult(ExecuteResult),
    ErrorOutput(ErrorOutput),
    CommOpen(CommOpen),
    CommMsg(CommMsg),
    CommClose(CommClose),
    CommInfoRequest(CommInfoRequest),
    CommInfoReply(CommInfoReply),
    ShutdownRequest(ShutdownRequest),
    ShutdownReply(ShutdownReply),
    InputRequest(InputRequest),
    InputReply(InputReply),
    InterruptRequest(InterruptRequest),
    InterruptReply(InterruptReply),
    CompleteRequest(CompleteRequest),
    CompleteReply(CompleteReply),
    HistoryRequest(HistoryRequest),
    HistoryReply(HistoryReply),
    IsCompleteRequest(IsCompleteRequest),
    IsCompleteReply(IsCompleteReply),
    Status(Status),
    UnknownMessage(UnknownMessage),
}

impl JupyterMessageContent {
    pub fn message_type(&self) -> &str {
        match self {
            JupyterMessageContent::ExecuteRequest(_) => "execute_request",
            JupyterMessageContent::ExecuteReply(_) => "execute_reply",
            JupyterMessageContent::KernelInfoRequest(_) => "kernel_info_request",
            JupyterMessageContent::KernelInfoReply(_) => "kernel_info_reply",
            JupyterMessageContent::StreamContent(_) => "stream",
            JupyterMessageContent::DisplayData(_) => "display_data",
            JupyterMessageContent::UpdateDisplayData(_) => "update_display_data",
            JupyterMessageContent::ExecuteInput(_) => "execute_input",
            JupyterMessageContent::ExecuteResult(_) => "execute_result",
            JupyterMessageContent::ErrorOutput(_) => "error",
            JupyterMessageContent::CommOpen(_) => "comm_open",
            JupyterMessageContent::CommMsg(_) => "comm_msg",
            JupyterMessageContent::CommClose(_) => "comm_close",
            JupyterMessageContent::CommInfoRequest(_) => "comm_info_request",
            JupyterMessageContent::CommInfoReply(_) => "comm_info_reply",
            JupyterMessageContent::ShutdownRequest(_) => "shutdown_request",
            JupyterMessageContent::ShutdownReply(_) => "shutdown_reply",
            JupyterMessageContent::InterruptRequest(_) => "interrupt_request",
            JupyterMessageContent::InterruptReply(__) => "interrupt_reply",
            JupyterMessageContent::InputRequest(_) => "input_request",
            JupyterMessageContent::InputReply(_) => "input_reply",
            JupyterMessageContent::CompleteRequest(_) => "complete_request",
            JupyterMessageContent::CompleteReply(_) => "complete_reply",
            JupyterMessageContent::HistoryRequest(_) => "history_request",
            JupyterMessageContent::HistoryReply(_) => "history_reply",
            JupyterMessageContent::IsCompleteRequest(_) => "is_complete_request",
            JupyterMessageContent::IsCompleteReply(_) => "is_complete_reply",
            JupyterMessageContent::Status(_) => "status",
            JupyterMessageContent::UnknownMessage(unk) => unk.msg_type.as_str(),
        }
    }

    pub fn from_type_and_content(msg_type: &str, content: Value) -> serde_json::Result<Self> {
        match msg_type {
            "execute_request" => Ok(JupyterMessageContent::ExecuteRequest(
                serde_json::from_value(content)?,
            )),
            "execute_input" => Ok(JupyterMessageContent::ExecuteInput(serde_json::from_value(
                content,
            )?)),
            "execute_reply" => Ok(JupyterMessageContent::ExecuteReply(serde_json::from_value(
                content,
            )?)),
            "kernel_info_request" => Ok(JupyterMessageContent::KernelInfoRequest(
                serde_json::from_value(content)?,
            )),
            "kernel_info_reply" => Ok(JupyterMessageContent::KernelInfoReply(
                serde_json::from_value(content)?,
            )),
            "stream" => Ok(JupyterMessageContent::StreamContent(
                serde_json::from_value(content)?,
            )),
            "display_data" => Ok(JupyterMessageContent::DisplayData(serde_json::from_value(
                content,
            )?)),
            "update_display_data" => Ok(JupyterMessageContent::UpdateDisplayData(
                serde_json::from_value(content)?,
            )),
            "execute_result" => Ok(JupyterMessageContent::ExecuteResult(
                serde_json::from_value(content)?,
            )),
            "error" => Ok(JupyterMessageContent::ErrorOutput(serde_json::from_value(
                content,
            )?)),
            "comm_open" => Ok(JupyterMessageContent::CommOpen(serde_json::from_value(
                content,
            )?)),
            "comm_msg" => Ok(JupyterMessageContent::CommMsg(serde_json::from_value(
                content,
            )?)),
            "comm_close" => Ok(JupyterMessageContent::CommClose(serde_json::from_value(
                content,
            )?)),
            "comm_info_request" => Ok(JupyterMessageContent::CommInfoRequest(
                serde_json::from_value(content)?,
            )),
            "comm_info_reply" => Ok(JupyterMessageContent::CommInfoReply(
                serde_json::from_value(content)?,
            )),
            "shutdown_request" => Ok(JupyterMessageContent::ShutdownRequest(
                serde_json::from_value(content)?,
            )),
            "shutdown_reply" => Ok(JupyterMessageContent::ShutdownReply(
                serde_json::from_value(content)?,
            )),

            "input_request" => Ok(JupyterMessageContent::InputRequest(serde_json::from_value(
                content,
            )?)),

            "input_reply" => Ok(JupyterMessageContent::InputReply(serde_json::from_value(
                content,
            )?)),

            "complete_request" => Ok(JupyterMessageContent::CompleteRequest(
                serde_json::from_value(content)?,
            )),

            "complete_reply" => Ok(JupyterMessageContent::CompleteReply(
                serde_json::from_value(content)?,
            )),

            "history_request" => Ok(JupyterMessageContent::HistoryRequest(
                serde_json::from_value(content)?,
            )),

            "history_reply" => Ok(JupyterMessageContent::HistoryReply(serde_json::from_value(
                content,
            )?)),

            "is_complete_request" => Ok(JupyterMessageContent::IsCompleteRequest(
                serde_json::from_value(content)?,
            )),

            "is_complete_reply" => Ok(JupyterMessageContent::IsCompleteReply(
                serde_json::from_value(content)?,
            )),

            "status" => Ok(JupyterMessageContent::Status(serde_json::from_value(
                content,
            )?)),

            _ => Ok(JupyterMessageContent::Unknown { msg_type, content }),
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
            fn as_child_of(self, parent: &JupyterMessage) -> JupyterMessage {
                JupyterMessage::new(JupyterMessageContent::$variant(self), Some(parent))
            }
        }

        impl From<$content_type> for JupyterMessage {
            fn from(content: $content_type) -> Self {
                JupyterMessage::new(JupyterMessageContent::$variant(content), None)
            }
        }
    };
}

impl_as_child_of!(ExecuteRequest, ExecuteRequest);
impl_as_child_of!(ExecuteReply, ExecuteReply);
impl_as_child_of!(KernelInfoRequest, KernelInfoRequest);
impl_as_child_of!(KernelInfoReply, KernelInfoReply);
impl_as_child_of!(StreamContent, StreamContent);
impl_as_child_of!(DisplayData, DisplayData);
impl_as_child_of!(UpdateDisplayData, UpdateDisplayData);
impl_as_child_of!(ExecuteInput, ExecuteInput);
impl_as_child_of!(ExecuteResult, ExecuteResult);
impl_as_child_of!(ErrorOutput, ErrorOutput);
impl_as_child_of!(CommOpen, CommOpen);
impl_as_child_of!(CommMsg, CommMsg);
impl_as_child_of!(CommClose, CommClose);
impl_as_child_of!(CommInfoRequest, CommInfoRequest);
impl_as_child_of!(CommInfoReply, CommInfoReply);
impl_as_child_of!(CompleteReply, CompleteReply);
impl_as_child_of!(IsCompleteReply, IsCompleteReply);
impl_as_child_of!(InputReply, InputReply);
impl_as_child_of!(HistoryReply, HistoryReply);
impl_as_child_of!(Status, Status);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnknownMessage {
    #[serde(skip_serializing, skip_deserializing)]
    pub msg_type: String,
    #[serde(flatten)]
    pub content: Value,
}

impl UnknownMessage {
    pub fn reply(&self) -> JupyterMessage {
        let message_content = JupyterMessageContent::UnknownMessage(UnknownMessage {
            msg_type: self.msg_type.replace("_request", "_reply"),
            content: json!({
                    "status": "ok",
            }),
        });

        // Must craft a very raw message
        JupyterMessage::new(message_content, Some(self))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteReply {
    pub status: String,
    pub execution_count: usize,
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
pub enum StdioMsg {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamContent {
    pub name: StdioMsg,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transient {
    pub display_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisplayData {
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Option<Transient>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateDisplayData {
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Transient,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteInput {
    pub code: String,
    pub execution_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteResult {
    pub execution_count: usize,
    pub data: MimeBundle,
    pub metadata: HashMap<String, String>,
    pub transient: Option<Transient>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorOutput {
    pub ename: String,
    pub evalue: String,
    pub traceback: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommOpen {
    pub comm_id: String,
    pub target_name: String,
    pub data: HashMap<String, Value>,
}

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

//

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
