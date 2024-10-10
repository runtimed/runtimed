use reqwest::Client;
use serde_json::Value;
use std::env;
use std::process::exit;
use tokio;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <jupyter-server-url>", args[0]);
        exit(1);
    }

    let jupyter_url = &args[1];
    let parsed_url = Url::parse(jupyter_url)?;

    // Extract the base URL and token
    let base_url = format!(
        "{}://{}{}",
        parsed_url.scheme(),
        parsed_url.host_str().unwrap_or("localhost"),
        parsed_url.path().trim_end_matches("/tree")
    );

    let token = parsed_url
        .query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned())
        .ok_or("Token not found in URL")?;

    let kernels_url = format!("{}/api/kernels", base_url);

    let client = Client::new();
    let response = client
        .get(&kernels_url)
        .header("Authorization", format!("Token {}", token))
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!(
            "Failed to connect to Jupyter server. Status: {}",
            response.status()
        );
        exit(1);
    }

    let kernels: Vec<Value> = response.json().await?;

    println!("Available kernels:");
    for kernel in kernels {
        println!(
            "ID: {}, Name: {}",
            kernel["id"].as_str().unwrap_or("Unknown"),
            kernel["name"].as_str().unwrap_or("Unknown")
        );
    }

    Ok(())
}
