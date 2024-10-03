
use std::env;
use std::process::exit;
use reqwest::blocking::Client;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <jupyter-server-url>", args[0]);
        exit(1);
    }

    let jupyter_url = &args[1];
    let kernels_url = format!("{}/api/kernels", jupyter_url);

    let client = Client::new();
    let response = client.get(&kernels_url).send()?;

    if !response.status().is_success() {
        eprintln!("Failed to connect to Jupyter server. Status: {}", response.status());
        exit(1);
    }

    let kernels: Vec<Value> = response.json()?;

    println!("Available kernels:");
    for kernel in kernels {
        println!("ID: {}, Name: {}", 
            kernel["id"].as_str().unwrap_or("Unknown"),
            kernel["name"].as_str().unwrap_or("Unknown")
        );
    }

    Ok(())
}
