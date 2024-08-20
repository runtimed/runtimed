// This file is forked/ported from <https://github.com/denoland/deno> which was
// originally forked from <https://github.com/evcxr/evcxr>

// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 The Evcxr Authors. MIT license.

use anyhow::anyhow;
use anyhow::bail;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use data_encoding::HEXLOWER;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::{json, Value};
use std::fmt;
use uuid::Uuid;
use zeromq::SocketRecv as _;
use zeromq::SocketSend as _;

mod time;

pub mod content;
pub use content::*;

type KernelIoPubSocket = zeromq::PubSocket;
type KernelShellSocket = zeromq::RouterSocket;
type KernelControlSocket = zeromq::RouterSocket;
type KernelStdinSocket = zeromq::RouterSocket;
type KernelHeartbeatSocket = zeromq::RepSocket;

type ClientIoPubSocket = zeromq::SubSocket;
type ClientShellSocket = zeromq::DealerSocket;
type ClientControlSocket = zeromq::DealerSocket;
type ClientStdinSocket = zeromq::DealerSocket;
type ClientHeartbeatSocket = zeromq::ReqSocket;

pub type KernelIoPubConnection = Connection<KernelIoPubSocket>;
pub type KernelShellConnection = Connection<KernelShellSocket>;
pub type KernelControlConnection = Connection<KernelControlSocket>;
pub type KernelStdinConnection = Connection<KernelStdinSocket>;
pub struct KernelHeartbeatConnection {
    pub socket: KernelHeartbeatSocket,
}

pub type ClientIoPubConnection = Connection<ClientIoPubSocket>;
pub type ClientShellConnection = Connection<ClientShellSocket>;
pub type ClientControlConnection = Connection<ClientControlSocket>;
pub type ClientStdinConnection = Connection<ClientStdinSocket>;
pub struct ClientHeartbeatConnection {
    pub socket: ClientHeartbeatSocket,
}

pub struct Connection<S> {
    pub socket: S,
    /// Will be None if our key was empty (digest authentication disabled).
    pub mac: Option<hmac::Key>,
    pub session_id: String,
}

impl<S: zeromq::Socket> Connection<S> {
    pub fn new(socket: S, key: &str, session_id: &str) -> Self {
        let mac = if key.is_empty() {
            None
        } else {
            Some(hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes()))
        };
        Connection {
            socket,
            mac,
            session_id: session_id.to_string(),
        }
    }
}

impl<S: zeromq::SocketSend> Connection<S> {
    pub async fn send(&mut self, message: JupyterMessage) -> Result<(), anyhow::Error> {
        let message = message.with_session(&self.session_id);
        let raw_message: RawMessage = message.into_raw_message()?;
        let zmq_message = raw_message.into_zmq_message(&self.mac)?;

        self.socket.send(zmq_message).await?;
        Ok(())
    }
}

impl<S: zeromq::SocketRecv> Connection<S> {
    pub async fn read(&mut self) -> Result<JupyterMessage, anyhow::Error> {
        let raw_message = RawMessage::from_multipart(self.socket.recv().await?, &self.mac)?;
        let message = JupyterMessage::from_raw_message(raw_message)?;
        Ok(message)
    }
}

impl KernelHeartbeatConnection {
    pub async fn single_heartbeat(&mut self) -> Result<(), anyhow::Error> {
        let _msg = self.socket.recv().await?;
        self.socket
            .send(zeromq::ZmqMessage::from(b"pong".to_vec()))
            .await?;
        Ok(())
    }
}

impl ClientHeartbeatConnection {
    pub async fn single_heartbeat(&mut self) -> Result<(), anyhow::Error> {
        self.socket
            .send(zeromq::ZmqMessage::from(b"ping".to_vec()))
            .await?;
        let _msg = self.socket.recv().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RawMessage {
    zmq_identities: Vec<Bytes>,
    jparts: Vec<Bytes>,
}

impl RawMessage {
    pub fn from_multipart(
        multipart: zeromq::ZmqMessage,
        key: &Option<hmac::Key>,
    ) -> Result<RawMessage, anyhow::Error> {
        let delimiter_index = multipart
            .iter()
            .position(|part| &part[..] == DELIMITER)
            .ok_or_else(|| anyhow!("Missing delimiter"))?;
        let mut parts = multipart.into_vec();
        let jparts: Vec<_> = parts.drain(delimiter_index + 2..).collect();
        let expected_hmac = parts.pop().ok_or_else(|| anyhow!("Missing hmac"))?;
        // Remove delimiter, so that what's left is just the identities.
        parts.pop();
        let zmq_identities = parts;

        let raw_message = RawMessage {
            zmq_identities,
            jparts,
        };

        if let Some(key) = key {
            let sig = HEXLOWER.decode(&expected_hmac)?;
            let mut msg = Vec::new();
            for part in &raw_message.jparts {
                msg.extend(part);
            }

            if let Err(err) = hmac::verify(key, msg.as_ref(), sig.as_ref()) {
                bail!("{}", err);
            }
        }

        Ok(raw_message)
    }

    fn hmac(&self, key: &Option<hmac::Key>) -> String {
        let hmac = if let Some(key) = key {
            let ctx = self.digest(key);
            let tag = ctx.sign();
            HEXLOWER.encode(tag.as_ref())
        } else {
            String::new()
        };
        hmac
    }

    fn digest(&self, mac: &hmac::Key) -> hmac::Context {
        let mut hmac_ctx = hmac::Context::with_key(mac);
        for part in &self.jparts {
            hmac_ctx.update(part);
        }
        hmac_ctx
    }

    fn into_zmq_message(
        self,
        key: &Option<hmac::Key>,
    ) -> Result<zeromq::ZmqMessage, anyhow::Error> {
        let hmac = self.hmac(key);

        let mut parts: Vec<bytes::Bytes> = Vec::new();
        for part in &self.zmq_identities {
            parts.push(part.to_vec().into());
        }
        parts.push(DELIMITER.into());
        parts.push(hmac.as_bytes().to_vec().into());
        for part in &self.jparts {
            parts.push(part.to_vec().into());
        }
        // ZmqMessage::try_from only fails if parts is empty, which it never
        // will be here.
        let message = zeromq::ZmqMessage::try_from(parts).map_err(|err| anyhow::anyhow!(err))?;
        Ok(message)
    }
}

#[derive(Serialize, Clone)]
pub struct JupyterMessage {
    #[serde(skip_serializing)]
    zmq_identities: Vec<Bytes>,
    pub header: Header,
    #[serde(serialize_with = "serialize_parent_header")]
    pub parent_header: Option<Header>,
    pub metadata: Value,
    pub content: JupyterMessageContent,
    #[serde(skip_serializing)]
    pub buffers: Vec<Bytes>,
}

/// Serializes the `parent_header`.
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub msg_id: String,
    pub username: String,
    pub session: String,
    pub date: DateTime<Utc>,
    pub msg_type: String,
    pub version: String,
}

const DELIMITER: &[u8] = b"<IDS|MSG>";

impl JupyterMessage {
    fn from_raw_message(raw_message: RawMessage) -> Result<JupyterMessage, anyhow::Error> {
        if raw_message.jparts.len() < 4 {
            // Be explicit with error here
            return Err(anyhow!(
                "Insufficient message parts {}",
                raw_message.jparts.len()
            ));
        }

        let header: Header = serde_json::from_slice(&raw_message.jparts[0])?;
        let content: Value = serde_json::from_slice(&raw_message.jparts[3])?;

        let content = JupyterMessageContent::from_type_and_content(&header.msg_type, content);

        let content = match content {
            Ok(content) => content,
            Err(err) => {
                return Err(anyhow!(
                    "Error deserializing content for msg_type `{}`: {}",
                    &header.msg_type,
                    err
                ));
            }
        };

        let parent_header = serde_json::from_slice(&raw_message.jparts[1]).ok();

        let message = JupyterMessage {
            zmq_identities: raw_message.zmq_identities,
            header,
            parent_header,
            metadata: serde_json::from_slice(&raw_message.jparts[2])?,
            content,
            buffers: if raw_message.jparts.len() > 4 {
                raw_message.jparts[4..].to_vec()
            } else {
                vec![]
            },
        };

        Ok(message)
    }

    pub fn message_type(&self) -> &str {
        self.content.message_type()
    }

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

    pub fn into_raw_message(&self) -> Result<RawMessage, anyhow::Error> {
        let mut jparts: Vec<Bytes> = vec![
            serde_json::to_vec(&self.header)?.into(),
            if let Some(parent_header) = self.parent_header.as_ref() {
                serde_json::to_vec(parent_header)?.into()
            } else {
                serde_json::to_vec(&serde_json::Map::new())?.into()
            },
            serde_json::to_vec(&self.metadata)?.into(),
            serde_json::to_vec(&self.content)?.into(),
        ];
        jparts.extend_from_slice(&self.buffers);
        let raw_message = RawMessage {
            zmq_identities: self.zmq_identities.clone(),
            jparts,
        };
        Ok(raw_message)
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
