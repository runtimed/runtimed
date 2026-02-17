//! Add a cell to a notebook.
//!
//! Usage:
//!   cargo run -p jupyter-ysync --features client --example add_cell

use jupyter_ysync::{NotebookSession, SessionConfig};

const BASE_URL: &str = "http://localhost:18889";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "agent_test.ipynb";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to notebook: {}", NOTEBOOK_PATH);

    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH).with_token(TOKEN);
    let mut session = NotebookSession::connect(config).await?;

    println!("Connected! Cell count: {}", session.cell_count());

    // Add a new code cell that multiplies a random value by 67
    let cell_id = uuid::Uuid::new_v4().to_string();
    session.doc().add_cell(
        &cell_id,
        jupyter_ysync::cell_types::CODE,
        "import random\nrandom.random() * 67",
        None,
    )?;

    println!("Added new cell with source: import random\\nrandom.random() * 67");

    // Sync the change to the server
    session.sync_to_server().await?;
    println!("Synced to server!");

    // Wait briefly for the update to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("Done! Check JupyterLab - new cell should appear.");

    session.close().await?;
    Ok(())
}
