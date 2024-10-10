use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::process::exit;
use tokio;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct KernelInfo {
    id: String,
    name: String,
    last_activity: String,
    execution_state: String,
    connections: i32,
}

#[derive(Serialize, Deserialize)]
struct JupyterMessage {
    header: Header,
    parent_header: Value,
    metadata: Value,
    content: Value,
}

#[derive(Serialize, Deserialize)]
struct Header {
    msg_id: String,
    username: String,
    session: String,
    msg_type: String,
    version: String,
}

async fn start_kernel(
    client: &Client,
    kernels_url: &str,
    token: &str,
) -> Result<KernelInfo, Box<dyn std::error::Error>> {
    let response = client
        .post(kernels_url)
        .header("Authorization", format!("Token {}", token))
        .json(&serde_json::json!({"name": "python3"}))
        .send()
        .await?;

    let kernel_info: KernelInfo = response.json().await?;
    Ok(kernel_info)
}

async fn connect_to_kernel(
    base_url: &str,
    kernel_id: &str,
    token: &str,
) -> Result<
    async_tungstenite::WebSocketStream<async_tungstenite::tokio::ConnectStream>,
    Box<dyn std::error::Error>,
> {
    let ws_url = format!(
        "{}/api/kernels/{}/channels?token={}",
        base_url.replace("http", "ws"),
        kernel_id,
        token
    );
    let url = url::Url::parse(&ws_url)?;

    let (ws_stream, response) = connect_async(url).await?;

    if response.status() != 101 {
        return Err(format!(
            "WebSocket connection failed with status: {}",
            response.status()
        )
        .into());
    }

    Ok(ws_stream)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <jupyter-server-url>", args[0]);
        exit(1);
    }

    let jupyter_url = &args[1];
    let parsed_url = Url::parse(jupyter_url)?;

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
        .ok_or("Token not found in URL")?;

    let kernels_url = format!("{}/api/kernels", base_url);

    println!("Connecting to: {}", kernels_url);

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    // Start a new kernel
    let kernel_info = start_kernel(&client, &kernels_url, &token).await?;
    println!("Started kernel: {:?}", kernel_info);

    // Connect to the kernel via WebSocket
    let mut ws_stream = connect_to_kernel(&base_url, &kernel_info.id, &token).await?;
    println!("Connected to kernel via WebSocket");

    // Send kernel_info_request
    let request_id = Uuid::new_v4().to_string();
    let kernel_info_request = JupyterMessage {
        header: Header {
            msg_id: request_id.clone(),
            username: "test".to_string(),
            session: Uuid::new_v4().to_string(),
            msg_type: "kernel_info_request".to_string(),
            version: "5.0".to_string(),
        },
        parent_header: serde_json::json!({}),
        metadata: serde_json::json!({}),
        content: serde_json::json!({}),
    };

    let request_str = serde_json::to_string(&kernel_info_request)?;
    println!("Sending kernel_info_request: {}", request_str);
    ws_stream.send(Message::Text(request_str)).await?;

    // Receive kernel_info_reply
    let mut received_reply = false;
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received message: {}", text);
                let response: Value = serde_json::from_str(&text)?;
                if response["parent_header"]["msg_id"] == request_id {
                    println!("Received kernel_info_reply: {:?}", response);
                    received_reply = true;
                    break;
                }
            }
            Ok(other) => println!("Received other message: {:?}", other),
            Err(e) => eprintln!("Error receiving message: {:?}", e),
        }
    }

    if !received_reply {
        println!("Did not receive kernel_info_reply");
    }

    // Shutdown the kernel
    let shutdown_response = client
        .delete(format!("{}/{}", kernels_url, kernel_info.id))
        .header("Authorization", format!("Token {}", token))
        .send()
        .await?;

    if shutdown_response.status().is_success() {
        println!("Kernel shut down successfully");
    } else {
        println!(
            "Failed to shut down kernel: {:?}",
            shutdown_response.status()
        );
    }

    Ok(())
}
