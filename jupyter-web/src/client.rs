use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::websocket::JupyterWebSocket;

pub struct JupyterClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

// Only `id` is used right now, but other fields will be useful when pulling up a listing later
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Kernel {
    pub id: String,
    pub name: String,
    pub last_activity: String,
    pub execution_state: String,
    pub connections: u64,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub session_type: String,
    pub kernel: Kernel,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct NewSession {
    pub path: String,
    pub name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub spec: KernelSpecFile,
    pub resources: std::collections::HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct KernelLaunch {
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct KernelSpecFile {
    pub argv: Vec<String>,
    pub display_name: String,
    pub language: String,
    pub codemirror_mode: Option<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub help_links: Option<Vec<HelpLink>>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct HelpLink {
    pub text: String,
    pub url: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct KernelSpecsResponse {
    pub default: String,
    pub kernelspecs: std::collections::HashMap<String, KernelSpec>,
}

impl JupyterClient {
    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/api/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    pub fn from_url(url: &str) -> Result<Self> {
        let parsed_url = Url::parse(url).context("Failed to parse Jupyter URL")?;
        let base_url = format!(
            "{}://{}{}{}",
            parsed_url.scheme(),
            parsed_url.host_str().unwrap_or("localhost"),
            parsed_url
                .port()
                .map(|p| format!(":{}", p))
                .unwrap_or_default(),
            parsed_url.path().trim_end_matches("/tree")
        );

        let token = parsed_url
            .query_pairs()
            .find(|(key, _)| key == "token")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| anyhow::anyhow!("Token not found in URL"))?;

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url,
            token,
        })
    }
    pub async fn start_kernel(&self, name: &str) -> Result<String> {
        let kernels_url = self.api_url("kernels");
        let response = self
            .client
            .post(&kernels_url)
            .header("Authorization", format!("Token {}", self.token))
            .json(&KernelLaunch {
                name: name.to_string(),
            })
            .send()
            .await
            .context("Failed to send kernel start request")?;

        let kernel = response
            .json::<Kernel>()
            .await
            .context("Failed to parse kernel info")?;

        eprintln!("Kernel info: {:?}", kernel);
        Ok(kernel.id)
    }
    pub async fn connect_to_kernel(&self, kernel_id: &str) -> Result<JupyterWebSocket> {
        let ws_url = format!(
            "{}?token={}",
            self.api_url(&format!("kernels/{}/channels", kernel_id))
                .replace("http", "ws"),
            self.token
        );

        let jupyter_websocket = crate::websocket::connect(&ws_url).await?;
        Ok(jupyter_websocket)
    }
    pub async fn shutdown(&self, kernel_id: &str) -> Result<()> {
        let kernels_url = self.api_url(&format!("kernels/{}", kernel_id));
        let response = self
            .client
            .delete(&kernels_url)
            .header("Authorization", format!("Token {}", self.token))
            .send()
            .await
            .context("Failed to send shutdown request")?;
        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to shut down kernel: {:?}", response.status())
        }
    }
    pub async fn list_kernels(&self) -> Result<Vec<Kernel>> {
        let url = self.api_url("kernels");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .send()
            .await
            .context("Failed to send list kernels request")?;

        response
            .json()
            .await
            .context("Failed to parse kernels list")
    }
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let url = self.api_url("sessions");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .send()
            .await
            .context("Failed to send list sessions request")?;

        response
            .json()
            .await
            .context("Failed to parse sessions list")
    }
    pub async fn list_kernel_specs(&self) -> Result<KernelSpecsResponse> {
        let url = self.api_url("kernelspecs");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .send()
            .await
            .context("Failed to send list kernel specs request")?;

        response
            .json()
            .await
            .context("Failed to parse kernel specs")
    }
}
