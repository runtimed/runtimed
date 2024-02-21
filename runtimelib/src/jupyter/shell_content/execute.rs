/*

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#execute
*/

use std::collections::HashMap;

use crate::jupyter::header::Header;
use crate::jupyter::message::Message;
use crate::jupyter::request::Request;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

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
