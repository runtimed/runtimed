use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

/// Connection information for a Jupyter kernel, as represented in a
/// JSON connection file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionInfo {
    pub ip: String,
    pub transport: String,
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub key: String,
    pub signature_scheme: String,
    // Ignore if not present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_name: Option<String>,
}

/// Generate a random key in the style of Jupyter: "AAAAAAAA-AAAAAAAAAAAAAAAAAAAAAAAA"
/// (A comment in the Python source indicates the author intended a dash
/// every 8 characters, but only actually does it for the first chunk)
pub fn jupyter_style_key() -> String {
    let a: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    let b: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    format!("{}-{}", a, b,)
}

fn form_url(transport: &str, ip: &str, port: u16) -> String {
    format!("{}://{}:{}", transport, ip, port)
}

impl ConnectionInfo {
    /// format the iopub url for a ZeroMQ connection
    pub fn iopub_url(&self) -> String {
        form_url(&self.transport, &self.ip, self.iopub_port)
    }

    /// format the shell url for a ZeroMQ connection
    pub fn shell_url(&self) -> String {
        form_url(&self.transport, &self.ip, self.shell_port)
    }

    /// format the stdin url for a ZeroMQ connection
    pub fn stdin_url(&self) -> String {
        form_url(&self.transport, &self.ip, self.stdin_port)
    }

    /// format the control url for a ZeroMQ connection
    pub fn control_url(&self) -> String {
        form_url(&self.transport, &self.ip, self.control_port)
    }

    /// format the heartbeat url for a ZeroMQ connection
    pub fn hb_url(&self) -> String {
        form_url(&self.transport, &self.ip, self.hb_port)
    }
}
