/*
Python kernel-sidecar uses the jupyter_client.py library to handle the low-level work of converting
Jupyter Message spec into over-the-wire protocol for ZMQ. There wasn't a jupyter-client crate
that I wanted to use here, so WireProtocol serialization and deserialization is handled here.

Ref: https://jupyter-client.readthedocs.io/en/latest/messaging.html#the-wire-protocol
*/
use crate::jupyter::constants::EMPTY_DICT_BYTES;
use crate::jupyter::header::Header;
use bytes::Bytes;
use ring::hmac;
use serde::Serialize;
use zeromq::ZmqMessage;

#[derive(Debug)]
pub struct WireProtocol {
    identity: Bytes,
    delimiter: Bytes,
    hmac_signature: Bytes,
    pub header: Bytes,
    pub parent_header: Bytes,
    pub metadata: Bytes,
    pub content: Bytes,
}

impl WireProtocol {
    pub fn new<T: Serialize>(header: Header, content: T, hmac_signing_key: &str) -> Self {
        // Serialize header to JSON then bytes
        let header = Bytes::from(serde_json::to_vec(&header).expect("Failed to serialize header"));
        // Make parent_header and metadata both empty dicts serialized to json and then bytes
        let parent_header = EMPTY_DICT_BYTES.clone();
        let metadata = EMPTY_DICT_BYTES.clone();

        // If content is passed in, serialize and turn into bytes. Otherwise same as parent_header
        let content =
            Bytes::from(serde_json::to_vec(&content).expect("Failed to serialize content"));

        let identity = Bytes::from("kernel");
        let delimiter = Bytes::from("<IDS|MSG>");
        let key = Bytes::from(hmac_signing_key.to_owned());
        let hmac_signature = Self::gen_hmac_sig(&key, &header, &parent_header, &metadata, &content);
        WireProtocol {
            identity,
            delimiter,
            hmac_signature,
            header,
            parent_header,
            metadata,
            content,
        }
    }

    fn gen_hmac_sig(
        key: &Bytes,
        header: &Bytes,
        parent_header: &Bytes,
        metadata: &Bytes,
        content: &Bytes,
    ) -> Bytes {
        let key = hmac::Key::new(hmac::HMAC_SHA256, key);

        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(header);
        ctx.update(parent_header);
        ctx.update(metadata);
        ctx.update(content);

        let tag = ctx.sign();
        let signature = hex::encode(tag.as_ref());
        Bytes::from(signature)
    }
}

impl From<WireProtocol> for ZmqMessage {
    fn from(wire_protocol: WireProtocol) -> Self {
        let mut zmq_message = ZmqMessage::from(wire_protocol.identity);
        zmq_message.push_back(wire_protocol.delimiter);
        zmq_message.push_back(wire_protocol.hmac_signature);
        zmq_message.push_back(wire_protocol.header);
        zmq_message.push_back(wire_protocol.parent_header);
        zmq_message.push_back(wire_protocol.metadata);
        zmq_message.push_back(wire_protocol.content);
        zmq_message
    }
}

impl From<ZmqMessage> for WireProtocol {
    fn from(zmq_message: ZmqMessage) -> Self {
        let mut frames = zmq_message.into_vecdeque();
        // The number of frames coming back from different types of Kernels has been frustratingly
        // inconsistent. evcxr_jupyter (Rust) sometimes sends back 6 frames, skipping the identity
        // frame. irkernel (R) sometimes sends two frames worth of identity.
        let identity = match frames.len() {
            8 => {
                // only seen this extra frame in irkernel so far, first frame is bytes and
                // second frame is the string 'kernel'
                frames.pop_front().expect("Missing identity frame");
                frames.pop_front().expect("Missing identity frame")
            }
            7 => frames.pop_front().expect("Missing identity frame"),
            _ => Bytes::from("missing identity header"),
        };
        let delimiter = frames.pop_front().expect("Missing delimiter frame");
        let hmac_signature = frames.pop_front().expect("Missing hmac_signature frame");
        let header = frames.pop_front().expect("Missing header frame");
        let parent_header = frames.pop_front().expect("Missing parent_header frame");
        let metadata = frames.pop_front().expect("Missing metadata frame");
        let content = frames.pop_front().expect("Missing content frame");

        WireProtocol {
            identity,
            delimiter,
            hmac_signature,
            header,
            parent_header,
            metadata,
            content,
        }
    }
}
