use crate::jupyter::header::Header;
use crate::jupyter::message::Message;
use crate::jupyter::request::Request;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Execute Request is _the_ way to send code to the kernel for execution.
///
/// Ref [`execute_request`](https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteRequest {
    code: String,
    silent: bool,
    store_history: bool,
    user_expressions: HashMap<String, String>,
    allow_stdin: bool,
    stop_on_error: bool,
}

impl ExecuteRequest {
    pub fn new(code: String) -> Self {
        ExecuteRequest {
            code,
            silent: false,
            store_history: true,
            user_expressions: HashMap::new(),
            allow_stdin: true,
            stop_on_error: true,
        }
    }
}

impl From<ExecuteRequest> for Request {
    fn from(req: ExecuteRequest) -> Self {
        let msg = Message {
            header: Header::new("execute_request".to_owned()),
            parent_header: None,
            metadata: None,
            content: req,
        };
        Request::Execute(msg)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ExecuteReply {
    status: String,
    execution_count: u32,
}

impl From<Bytes> for ExecuteReply {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Failed to deserialize ExecuteReply")
    }
}

/*
Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-info
*/

// KernelInfoRequest, related sub-structs, and impls
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfoRequest {}

impl Default for KernelInfoRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelInfoRequest {
    pub fn new() -> Self {
        KernelInfoRequest {}
    }
}

impl From<KernelInfoRequest> for Request {
    fn from(req: KernelInfoRequest) -> Self {
        let msg = Message {
            header: Header::new("kernel_info_request".to_owned()),
            parent_header: None,
            metadata: None,
            content: req,
        };
        Request::KernelInfo(msg)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct HelpLink {
    text: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LanguageInfo {
    name: String,
    version: String,
    mimetype: String,
    file_extension: String,
    pygments_lexer: Option<String>,
    codemirror_mode: Option<serde_json::Value>,
    nbconvert_exporter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KernelInfoReply {
    banner: String,
    help_links: Option<Vec<HelpLink>>,
    implementation: String,
    implementation_version: String,
    language_info: LanguageInfo,
    protocol_version: String,
    status: String,
}

impl From<Bytes> for KernelInfoReply {
    fn from(bytes: Bytes) -> Self {
        let reply: KernelInfoReply = serde_json::from_slice(&bytes)
            .expect("Failed to deserialize ZMQ bytes to KernelInfoReply");
        reply
    }
}
