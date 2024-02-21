/*
For the most part, Message<T> isn't something you as a developer will use in application code.

For creating requests from client to kernel, the impl From<T> for Request is in the appropriate
message_content files, where Message<T> is used as part of that impl.

For deserialiing responses from kernel to client, the impl From<WireProtocol> for Response creates
the appropriate Message<T> based on the msg_type in the header.

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#a-full-message
*/
use crate::jupyter::header::Header;
use crate::jupyter::metadata::Metadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T> {
    pub header: Header,
    pub parent_header: Option<Header>,
    pub metadata: Option<Metadata>,
    pub content: T,
}

impl<T> Message<T> {
    pub fn parent_msg_id(&self) -> Option<String> {
        self.parent_header
            .as_ref()
            .map(|header| header.msg_id.to_owned())
    }

    pub fn msg_type(&self) -> String {
        self.header.msg_type.to_owned()
    }
}
