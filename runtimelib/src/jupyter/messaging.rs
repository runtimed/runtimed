// This file is forked/ported from <https://github.com/denoland/deno> which was
// originally forked from <https://github.com/evcxr/evcxr>

// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 The Evcxr Authors. MIT license.

use anyhow::anyhow;
use anyhow::bail;
use bytes::Bytes;
use chrono::Utc;
use data_encoding::HEXLOWER;
use ring::hmac;
use serde::{de, Deserialize, Deserializer};
use serde_json;
use serde_json::{json, Map, Value};
use std::fmt;
use uuid::Uuid;

use crate::jupyter::message_content::JupyterMessageContent;

pub(crate) struct Connection<S> {
    pub(crate) socket: S,
    /// Will be None if our key was empty (digest authentication disabled).
    pub(crate) mac: Option<hmac::Key>,
}

impl<S: zeromq::Socket> Connection<S> {
    pub(crate) fn new(socket: S, key: &str) -> Self {
        let mac = if key.is_empty() {
            None
        } else {
            Some(hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes()))
        };
        Connection { socket, mac }
    }
}

struct RawMessage {
    zmq_identities: Vec<Bytes>,
    jparts: Vec<Bytes>,
}

impl RawMessage {
    pub(crate) async fn read<S: zeromq::SocketRecv>(
        connection: &mut Connection<S>,
    ) -> Result<RawMessage, anyhow::Error> {
        Self::from_multipart(connection.socket.recv().await?, connection)
    }

    pub(crate) fn from_multipart<S>(
        multipart: zeromq::ZmqMessage,
        connection: &Connection<S>,
    ) -> Result<RawMessage, anyhow::Error> {
        let delimiter_index = multipart
            .iter()
            .position(|part| &part[..] == DELIMITER)
            .ok_or_else(|| anyhow!("Missing delimiter"))?;
        let mut parts = multipart.into_vec();
        let jparts: Vec<_> = parts.drain(delimiter_index + 2..).collect();
        let expected_hmac = parts.pop().unwrap();
        // Remove delimiter, so that what's left is just the identities.
        parts.pop();
        let zmq_identities = parts;

        let raw_message = RawMessage {
            zmq_identities,
            jparts,
        };

        if let Some(key) = &connection.mac {
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

    async fn send<S: zeromq::SocketSend>(
        self,
        connection: &mut Connection<S>,
    ) -> Result<(), anyhow::Error> {
        let hmac = if let Some(key) = &connection.mac {
            let ctx = self.digest(key);
            let tag = ctx.sign();
            HEXLOWER.encode(tag.as_ref())
        } else {
            String::new()
        };
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
        let message = zeromq::ZmqMessage::try_from(parts).unwrap();
        connection.socket.send(message).await?;
        Ok(())
    }

    fn digest(&self, mac: &hmac::Key) -> hmac::Context {
        let mut hmac_ctx = hmac::Context::with_key(mac);
        for part in &self.jparts {
            hmac_ctx.update(part);
        }
        hmac_ctx
    }
}

#[derive(serde::Serialize, Clone)]
pub struct JupyterMessage {
    #[serde(skip_serializing)]
    zmq_identities: Vec<Bytes>,
    pub header: serde_json::Value,
    pub parent_header: serde_json::Value,
    pub metadata: serde_json::Value,
    #[serde(deserialize_with = "deserialize_content")]
    pub content: JupyterMessageContent,
    #[serde(skip_serializing)]
    pub buffers: Vec<Bytes>,
}

const DELIMITER: &[u8] = b"<IDS|MSG>";

impl JupyterMessage {
    pub(crate) async fn read<S: zeromq::SocketRecv>(
        connection: &mut Connection<S>,
    ) -> Result<JupyterMessage, anyhow::Error> {
        Self::from_raw_message(RawMessage::read(connection).await?)
    }

    fn from_raw_message(raw_message: RawMessage) -> Result<JupyterMessage, anyhow::Error> {
        if raw_message.jparts.len() < 4 {
            bail!("Insufficient message parts {}", raw_message.jparts.len());
        }

        let header: serde_json::Value = serde_json::from_slice(&raw_message.jparts[0])?;
        let msg_type = header["msg_type"].as_str().unwrap_or("");
        let content: serde_json::Value = serde_json::from_slice(&raw_message.jparts[3])?;

        let content = JupyterMessageContent::from_type_and_content(msg_type, content);

        let content = match content {
            Ok(content) => content,
            Err(err) => {
                bail!("Error deserializing content: {}", err);
            }
        };

        Ok(JupyterMessage {
            zmq_identities: raw_message.zmq_identities,
            header,
            parent_header: serde_json::from_slice(&raw_message.jparts[1])?,
            metadata: serde_json::from_slice(&raw_message.jparts[2])?,
            content,
            buffers: if raw_message.jparts.len() > 4 {
                raw_message.jparts[4..].to_vec()
            } else {
                vec![]
            },
        })
    }

    pub(crate) fn message_type(&self) -> &str {
        self.header["msg_type"].as_str().unwrap_or("")
    }

    pub fn new(content: JupyterMessageContent) -> JupyterMessage {
        let header = json!({
            "msg_id": Uuid::new_v4().to_string(),
            "username": "runtimelib",
            "session": Uuid::new_v4().to_string(),
            "date": Utc::now().to_rfc3339(),
            "msg_type": content.message_type(),
            "version": "5.3",
        });

        JupyterMessage {
            zmq_identities: Vec::new(),
            header,
            parent_header: json!({}), // Empty for a new message
            metadata: json!({}),
            content,
            buffers: Vec::new(),
        }
    }

    pub(crate) async fn send<S: zeromq::SocketSend>(
        &self,
        connection: &mut Connection<S>,
    ) -> Result<(), anyhow::Error> {
        // If performance is a concern, we can probably avoid the clone and to_vec calls with a bit
        // of refactoring.
        let mut jparts: Vec<Bytes> = vec![
            serde_json::to_string(&self.header)
                .unwrap()
                .as_bytes()
                .to_vec()
                .into(),
            serde_json::to_string(&self.parent_header)
                .unwrap()
                .as_bytes()
                .to_vec()
                .into(),
            serde_json::to_string(&self.metadata)
                .unwrap()
                .as_bytes()
                .to_vec()
                .into(),
            serde_json::to_string(&self.content)
                .unwrap()
                .as_bytes()
                .to_vec()
                .into(),
        ];
        jparts.extend_from_slice(&self.buffers);
        let raw_message = RawMessage {
            zmq_identities: self.zmq_identities.clone(),
            jparts,
        };
        raw_message.send(connection).await
    }
}

// Dead because we never deserialize JupyterMessage directly (at the moment). It happens in
// slices
#[allow(dead_code)]
fn deserialize_content<'de, D>(deserializer: D) -> Result<JupyterMessageContent, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;
    let header: &Map<String, Value> = value
        .get("header")
        .and_then(Value::as_object)
        .ok_or(de::Error::custom("header is missing"))?;
    let msg_type: &str = header
        .get("msg_type")
        .and_then(Value::as_str)
        .ok_or(de::Error::custom("msg_type is missing"))?;
    let content_value = value
        .get("content")
        .ok_or(de::Error::custom("content is missing"))?
        .clone();

    let content = JupyterMessageContent::from_type_and_content(msg_type, content_value);
    if let Ok(content) = content {
        Ok(content)
    } else {
        Err(de::Error::custom("Unknown message type"))
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
            serde_json::to_string_pretty(&self.parent_header).unwrap()
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
