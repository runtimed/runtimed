/*
This file is all about serializing messages going from Client to Kernel.

message_content T -> Message<T> -> Request -> WireProtocol -> zeromq::ZmqMessage

The impl's for message_content T -> Message<T> -> Request are in individual message_content files
*/

use crate::jupyter::content::shell::{ExecuteRequest, KernelInfoRequest};
use crate::jupyter::message::{Message, MessageLike};
use crate::jupyter::wire_protocol::WireProtocol;

#[derive(Debug)]
pub enum Request {
    KernelInfo(Message<KernelInfoRequest>),
    Execute(Message<ExecuteRequest>),
}

impl Request {
    pub fn msg_id(&self) -> String {
        // return msg_id from header
        match self {
            Request::KernelInfo(msg) => msg.header.msg_id.to_owned(),
            Request::Execute(msg) => msg.header.msg_id.to_owned(),
        }
    }

    pub fn into_wire_protocol(&self, hmac_signing_key: &str) -> WireProtocol {
        match self {
            Request::KernelInfo(msg) => WireProtocol::new(
                msg.header.clone(),
                Some(msg.content.clone()),
                hmac_signing_key,
            ),
            Request::Execute(msg) => WireProtocol::new(
                msg.header.clone(),
                Some(msg.content.clone()),
                hmac_signing_key,
            ),
        }
    }
}
