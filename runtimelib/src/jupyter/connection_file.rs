/**
 * TODO: Convert to tokio for fs operations
 */
/**
 * TODO: Use OpenSSL to generate a key
 */
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::net::TcpListener;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionInfo {
    ip: String,
    transport: String,
    shell_port: u16,
    iopub_port: u16,
    stdin_port: u16,
    control_port: u16,
    hb_port: u16,
    signature_scheme: String,
    pub key: String,
    kernel_name: Option<String>,
}

fn find_open_port() -> Result<u16, std::io::Error> {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
}

fn generate_hmac_key() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

impl ConnectionInfo {
    pub fn new(kernel_name: Option<String>) -> Result<Self, io::Error> {
        let mut ports = HashSet::new();
        while ports.len() < 5 {
            let port = find_open_port()?;
            ports.insert(port);
        }

        let mut port_iter = ports.into_iter();
        Ok(Self {
            ip: "127.0.0.1".to_string(),
            transport: "tcp".to_string(),
            shell_port: port_iter.next().unwrap(),
            iopub_port: port_iter.next().unwrap(),
            stdin_port: port_iter.next().unwrap(),
            control_port: port_iter.next().unwrap(),
            hb_port: port_iter.next().unwrap(),
            signature_scheme: "hmac-sha256".to_string(),
            key: generate_hmac_key(),
            kernel_name,
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file_contents = fs::read_to_string(path)?;
        serde_json::from_str(&file_contents).map_err(io::Error::from)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn to_temp_file(&self) -> Result<std::path::PathBuf, io::Error> {
        let mut file_path = std::env::temp_dir();
        if self.kernel_name.is_some() {
            file_path.push(format!(
                "kernel-sidecar-{}-{}.json",
                self.kernel_name.as_ref().unwrap(),
                uuid::Uuid::new_v4()
            ));
        } else {
            file_path.push(format!("kernel-sidecar-{}.json", uuid::Uuid::new_v4()));
        }
        self.to_file(&file_path)?;
        Ok(file_path)
    }

    pub fn iopub_address(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.iopub_port)
    }

    pub fn shell_address(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.shell_port)
    }

    pub fn heartbeat_address(&self) -> String {
        format!("{}://{}:{}", self.transport, self.ip, self.hb_port)
    }
}
