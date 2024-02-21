/*
Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#kernel-info
*/

use crate::jupyter::header::Header;
use crate::jupyter::message::Message;
use crate::jupyter::request::Request;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

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

// KernelInfoReply, related sub-structs, and impls
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
