/*
This file is all about deserializing messages coming from Kernel to Client.

zeromq::ZmqMessage -> WireProtocol -> Response -> Message<T> with Jupyter message content T
*/
use crate::jupyter::constants::EMPTY_DICT_BYTES;
use crate::jupyter::content::iopub::{
    ClearOutput, DisplayData, Error, ExecuteInput, ExecuteResult, Status, Stream, UpdateDisplayData,
};
use crate::jupyter::content::shell::{ExecuteReply, KernelInfoReply};
use crate::jupyter::header::Header;
use crate::jupyter::message::{Message, Metadata};
use crate::jupyter::wire_protocol::WireProtocol;
use serde::{Deserialize, Serialize};

use zeromq::ZmqMessage;

#[derive(Debug, Serialize, Deserialize)]
pub struct UnmodeledContent(serde_json::Value);

#[derive(Debug)]
pub enum Response {
    // Request/reply from shell channel
    KernelInfo(Message<KernelInfoReply>),
    Execute(Message<ExecuteReply>),
    // Messages from iopub channel
    Status(Message<Status>),
    ExecuteInput(Message<ExecuteInput>),
    ExecuteResult(Message<ExecuteResult>),
    Stream(Message<Stream>),
    DisplayData(Message<DisplayData>),
    UpdateDisplayData(Message<UpdateDisplayData>),
    ClearOutput(Message<ClearOutput>),
    // Errors
    Error(Message<Error>),
    // Messages I haven't modeled yet, crate is WIP
    Unmodeled(Message<UnmodeledContent>),
}

impl Response {
    pub fn parent_msg_id(&self) -> Option<String> {
        // return parent_msg_id from header
        match self {
            Response::Status(msg) => msg.parent_msg_id(),
            Response::KernelInfo(msg) => msg.parent_msg_id(),
            Response::Execute(msg) => msg.parent_msg_id(),
            Response::ExecuteInput(msg) => msg.parent_msg_id(),
            Response::ExecuteResult(msg) => msg.parent_msg_id(),
            Response::Stream(msg) => msg.parent_msg_id(),
            Response::DisplayData(msg) => msg.parent_msg_id(),
            Response::UpdateDisplayData(msg) => msg.parent_msg_id(),
            Response::ClearOutput(msg) => msg.parent_msg_id(),
            Response::Error(msg) => msg.parent_msg_id(),
            Response::Unmodeled(msg) => msg.parent_msg_id(),
        }
    }

    pub fn msg_type(&self) -> String {
        // return msg_type from header
        match self {
            Response::Status(msg) => msg.header.msg_type.to_owned(),
            Response::KernelInfo(msg) => msg.header.msg_type.to_owned(),
            Response::Execute(msg) => msg.header.msg_type.to_owned(),
            Response::ExecuteInput(msg) => msg.header.msg_type.to_owned(),
            Response::ExecuteResult(msg) => msg.header.msg_type.to_owned(),
            Response::Stream(msg) => msg.header.msg_type.to_owned(),
            Response::DisplayData(msg) => msg.header.msg_type.to_owned(),
            Response::UpdateDisplayData(msg) => msg.header.msg_type.to_owned(),
            Response::ClearOutput(msg) => msg.header.msg_type.to_owned(),
            Response::Error(msg) => msg.header.msg_type.to_owned(),
            Response::Unmodeled(msg) => {
                let real_msg_type = msg.header.msg_type.to_owned();
                format!("unmodeled_{}", real_msg_type)
            }
        }
    }
}

impl From<WireProtocol> for Response {
    fn from(wp: WireProtocol) -> Self {
        let header: Header = wp.header.into();
        let parent_header = match wp.parent_header == EMPTY_DICT_BYTES.clone() {
            true => None,
            false => Some(wp.parent_header.into()),
        };
        let metadata: Metadata = wp.metadata.into();
        match header.msg_type.as_str() {
            "status" => {
                let content: Status = wp.content.into();
                let msg: Message<Status> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::Status(msg)
            }
            "kernel_info_reply" => {
                let content: KernelInfoReply = wp.content.into();
                let msg: Message<KernelInfoReply> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::KernelInfo(msg)
            }
            "execute_reply" => {
                let content: ExecuteReply = wp.content.into();
                let msg: Message<ExecuteReply> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::Execute(msg)
            }
            "execute_input" => {
                let content: ExecuteInput = wp.content.into();
                let msg: Message<ExecuteInput> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::ExecuteInput(msg)
            }
            "execute_result" => {
                let content: ExecuteResult = wp.content.into();
                let msg: Message<ExecuteResult> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::ExecuteResult(msg)
            }
            "stream" => {
                let content: Stream = wp.content.into();
                let msg: Message<Stream> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::Stream(msg)
            }
            "display_data" => {
                let content: DisplayData = wp.content.into();
                let msg: Message<DisplayData> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::DisplayData(msg)
            }
            "update_display_data" => {
                let content: UpdateDisplayData = wp.content.into();
                let msg: Message<UpdateDisplayData> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::UpdateDisplayData(msg)
            }
            "clear_output" => {
                let content: ClearOutput = wp.content.into();
                let msg: Message<ClearOutput> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::ClearOutput(msg)
            }
            "error" => {
                let content: Error = wp.content.into();
                let msg: Message<Error> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::Error(msg)
            }
            _ => {
                let content: UnmodeledContent = serde_json::from_slice(&wp.content)
                    .expect("Error deserializing unmodeled content");
                let msg: Message<UnmodeledContent> = Message {
                    header,
                    parent_header,
                    metadata: Some(metadata),
                    content,
                };
                Response::Unmodeled(msg)
            }
        }
    }
}

impl From<ZmqMessage> for Response {
    fn from(msg: ZmqMessage) -> Self {
        let wp: WireProtocol = msg.into();
        wp.into()
    }
}
