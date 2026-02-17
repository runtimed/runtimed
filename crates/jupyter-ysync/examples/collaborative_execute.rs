//! Collaborative execution example - connects to a notebook and executes cells.
//!
//! This demonstrates the high-level NotebookSession API that combines:
//! - Y-sync for real-time document synchronization
//! - Kernel WebSocket for code execution
//!
//! Usage:
//!   1. Start jupyter-server-documents:
//!      cd /tmp/jupyter-test && uv run --with jupyter-server --with jupyter-server-documents \
//!        --with jupyterlab jupyter lab --port 18888 --IdentityProvider.token=testtoken123
//!
//!   2. Create a notebook with at least one code cell
//!
//!   3. Run this example:
//!      cargo run -p jupyter-ysync --features client --example collaborative_execute

use jupyter_ysync::{NotebookSession, SessionConfig};

const BASE_URL: &str = "http://localhost:18888";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "test.ipynb";

/// Truncate a string to a maximum number of characters, adding "..." if truncated.
/// This is UTF-8 safe (truncates at char boundaries, not byte boundaries).
fn truncate_preview(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Collaborative Notebook Execution ===\n");

    // Create session configuration
    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH)
        .with_token(TOKEN);

    println!("Connecting to notebook: {}", NOTEBOOK_PATH);
    println!("Base URL: {}", BASE_URL);

    // Connect to the notebook (Y-sync only)
    let mut session = NotebookSession::connect(config).await?;
    println!("Connected to Y-sync room!");

    // Display notebook info
    let cell_count = session.cell_count();
    println!("\nNotebook has {} cells", cell_count);

    if cell_count == 0 {
        println!("\nNo cells in notebook. Please create at least one code cell.");
        session.close().await?;
        return Ok(());
    }

    // Show all cell sources
    for i in 0..cell_count {
        if let Some(source) = session.get_cell_source(i) {
            let preview = truncate_preview(&source, 50);
            println!("  Cell {}: {}", i, preview.replace('\n', "\\n"));
        }
    }

    // Connect to kernel
    println!("\nLaunching kernel...");
    session.connect_kernel(None).await?;
    println!("Kernel connected! ID: {:?}", session.kernel_id());

    // Execute first code cell
    println!("\nExecuting cell 0...");
    let events = session.execute_cell(0).await?;

    println!("\nExecution events:");
    for event in &events {
        println!("  {:?}", event);
    }

    // Sync any local changes (in case write_outputs_locally was enabled)
    session.sync_to_server().await?;

    // Wait a bit for server to process outputs
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("\nDone! Check JupyterLab to see the outputs.");

    // Clean up
    session.close().await?;
    println!("\nSession closed.");

    Ok(())
}
