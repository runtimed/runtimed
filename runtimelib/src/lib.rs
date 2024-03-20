//! `runtimelib` - Runtime Discovery and Management for Jupyter Environments
//!
//! This module provides functionality to discover, list, and attach to Jupyter runtime instances.
//! It includes mechanisms to validate and locate Jupyter connection files, which are essential
//! for establishing communication with running Jupyter kernels.
//!
//! # Examples
//!
//! Listing all available Jupyter runtime instances:
//!
//! ```no_run
//! # use runtimed::list_instances;
//! # async {
//! let instances = list_instances().await;
//! for instance in instances {
//!     println!("Found Jupyter runtime: {:?}", instance);
//! }
//! # };
//! ```
//!
//! Attaching to a specific Jupyter runtime instance by ID:
//!
//! ```no_run
//! # use runtimed::attach;
//! # use anyhow::Result;
//! # async {
//! let client: Result<_, _> = attach("some-runtime-id".to_string()).await;
//! match client {
//!     Ok(client) => println!("Attached to Jupyter runtime."),
//!     Err(e) => println!("Failed to attach: {}", e),
//! }
//! # };
//! ```
//!
//! # Note
//!
//! The `attach` function expects the ID to be a valid string containing only alphanumeric
//! characters, hyphens, and underscores. It will return an error if the provided ID does not
//! conform to this pattern.

pub mod jupyter;
use crate::jupyter::client;
use crate::jupyter::dirs;
use crate::jupyter::discovery;

pub mod environment;
pub mod media;
pub mod messaging;

use anyhow::anyhow;
use anyhow::Error;

use glob::glob;

pub async fn list_instances() -> Vec<client::JupyterRuntime> {
    discovery::get_jupyter_runtime_instances().await
}

pub async fn attach(id: String) -> Result<client::JupyterClient, Error> {
    // Goal: Attach to a running instance based on the connection file
    // See if {runtime_dir}/{id}.json exists (or {runtime_dir}/kernel-{id}.json) exists
    // Create a client from that connection info

    // Validate that id only contains alphanumeric characters, hyphens, and underscores
    // If it doesn't, return an error
    // If it does, continue

    // Validate ID format
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Invalid ID: only alphanumeric characters, hyphens, and underscores are allowed"
        ));
    }

    let runtime_dir = dirs::runtime_dir();

    // Prepare glob patterns
    let pattern1 = runtime_dir.join(format!("{id}.json"));
    let pattern2 = runtime_dir.join(format!("kernel-{id}.json"));

    // Convert PathBuf to String for glob
    let glob_pattern1 = pattern1
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert path to string"))?
        .to_string();
    let glob_pattern2 = pattern2
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert path to string"))?
        .to_string();

    // Search for matching files
    let mut found_files = Vec::new();
    for entry in glob(&glob_pattern1)
        .expect("Failed to read glob pattern")
        .chain(glob(&glob_pattern2).expect("Failed to read glob pattern"))
    {
        match entry {
            Ok(path) => found_files.push(path),
            Err(e) => println!("Error while matching glob pattern: {}", e),
        }
    }

    // Get the first found file and attach to it
    if let Some(file_path) = found_files.into_iter().next() {
        println!("Found runtime file: {:?}", file_path);

        let runtime = discovery::load_connection_file(file_path).await?;

        return runtime.attach().await;
    }

    Err(anyhow!("No matching runtime files found"))
}
