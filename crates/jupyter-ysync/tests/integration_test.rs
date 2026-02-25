#![cfg(feature = "client")]

//! Integration tests for Y-sync client against a real Jupyter server.
//!
//! # Prerequisites
//!
//! 1. Start a Jupyter server with jupyter-server-documents:
//!    ```bash
//!    cd /tmp/jupyter-test && uv run --with jupyter-server --with jupyter-server-documents \
//!      --with jupyterlab jupyter lab --port 18889 --IdentityProvider.token=testtoken123
//!    ```
//!
//! 2. Create `test.ipynb` and keep it open in JupyterLab (Y-sync room must be active)
//!
//! 3. Run tests:
//!    ```bash
//!    cargo test -p jupyter-ysync --features client -- --ignored --test-threads=1 --nocapture
//!    ```
//!
//! # Environment Variables
//!
//! - `JUPYTER_URL` - Base URL (default: http://localhost:18889)
//! - `JUPYTER_TOKEN` - Auth token (default: testtoken123)
//! - `NOTEBOOK_PATH` - Notebook path (default: test.ipynb)

use jupyter_ysync::{cell_types, ExecutionEvent, NotebookSession, SessionConfig};
use std::time::Duration;
use tokio::time::timeout;

const DEFAULT_URL: &str = "http://localhost:18889";
const DEFAULT_TOKEN: &str = "testtoken123";
const DEFAULT_NOTEBOOK: &str = "test.ipynb";
const TIMEOUT: Duration = Duration::from_secs(10);

fn get_config() -> SessionConfig {
    let url = std::env::var("JUPYTER_URL").unwrap_or_else(|_| DEFAULT_URL.into());
    let token = std::env::var("JUPYTER_TOKEN").unwrap_or_else(|_| DEFAULT_TOKEN.into());
    let notebook = std::env::var("NOTEBOOK_PATH").unwrap_or_else(|_| DEFAULT_NOTEBOOK.into());
    SessionConfig::new(&url, &notebook).with_token(&token)
}

#[tokio::test]
#[ignore] // Requires running Jupyter server
async fn test_connect_and_list_cells() {
    let config = get_config();
    let session = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    let cell_count = session.cell_count();
    println!("Notebook has {} cells", cell_count);
    assert!(cell_count > 0, "Notebook should have at least one cell");

    // Verify we can read cell sources
    for i in 0..cell_count {
        let source = session.get_cell_source(i);
        assert!(source.is_some(), "Cell {} should have source", i);
        let src = source.unwrap();
        let preview: String = src.chars().take(50).collect();
        println!("Cell {}: {}", i, preview.replace('\n', "\\n"));
    }

    session.close().await.expect("Failed to close session");
}

#[tokio::test]
#[ignore] // Requires running Jupyter server
async fn test_edit_cell_syncs_to_server() {
    let config = get_config();
    let mut session = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    // Store original content
    let original = session
        .get_cell_source(0)
        .expect("Cell 0 should exist")
        .clone();
    println!(
        "Original cell 0: {}",
        original.chars().take(30).collect::<String>()
    );

    // Edit cell with unique timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let new_content = format!("# Modified at {}\nprint('test')", timestamp);

    session
        .set_cell_source(0, &new_content)
        .expect("Failed to set source");
    session.sync_to_server().await.expect("Failed to sync");
    println!("Edited cell 0");

    // Close and reconnect to verify persistence
    session.close().await.expect("Failed to close");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let config = get_config();
    let mut session2 = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    let synced_content = session2.get_cell_source(0).expect("Cell 0 should exist");
    println!(
        "Synced cell 0: {}",
        synced_content.chars().take(50).collect::<String>()
    );
    assert!(
        synced_content.contains(&format!("{}", timestamp)),
        "Edit should have synced"
    );

    // Restore original content
    session2
        .set_cell_source(0, &original)
        .expect("Failed to restore");
    session2.sync_to_server().await.expect("Failed to sync");
    session2.close().await.expect("Failed to close");
    println!("Restored original content");
}

#[tokio::test]
#[ignore] // Requires running Jupyter server
async fn test_add_and_remove_cell() {
    let config = get_config();
    let mut session = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    let initial_count = session.cell_count();
    println!("Initial cell count: {}", initial_count);

    // Add new cell
    let cell_id = uuid::Uuid::new_v4().to_string();
    session
        .doc()
        .add_cell(
            &cell_id,
            cell_types::CODE,
            "# test cell from integration test",
            None,
        )
        .expect("Failed to add cell");
    session.sync_to_server().await.expect("Failed to sync");

    // Verify locally
    assert_eq!(
        session.cell_count(),
        initial_count + 1,
        "Cell count should increase"
    );
    println!("Added cell, new count: {}", session.cell_count());

    // Close and reconnect to verify
    session.close().await.expect("Failed to close");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let config = get_config();
    let mut session2 = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    assert_eq!(
        session2.cell_count(),
        initial_count + 1,
        "Cell should persist after reconnect"
    );

    // Clean up: remove the added cell
    session2
        .doc()
        .remove_cell(initial_count)
        .expect("Failed to remove cell");
    session2.sync_to_server().await.expect("Failed to sync");
    println!("Removed test cell");

    session2.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires running Jupyter server
async fn test_execute_cell() {
    let config = get_config();
    let mut session = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    // Set up a simple cell to execute
    session
        .set_cell_source(0, "print('hello from integration test')")
        .expect("Failed to set source");
    session.sync_to_server().await.expect("Failed to sync");
    println!("Set up cell for execution");

    // Connect kernel and execute
    session
        .connect_kernel(None)
        .await
        .expect("Failed to connect kernel");
    println!("Kernel connected: {:?}", session.kernel_id());

    let events = timeout(Duration::from_secs(30), session.execute_cell(0))
        .await
        .expect("Execution timed out")
        .expect("Failed to execute");

    println!("Execution events:");
    for event in &events {
        println!("  {:?}", event);
    }

    // Verify execution completed
    let completed = events
        .iter()
        .any(|e| matches!(e, ExecutionEvent::Completed { .. }));
    assert!(completed, "Execution should complete");

    session.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires running Jupyter server
async fn test_collaborative_scenario() {
    // This test simulates the human + agent collaborative editing scenario
    println!("=== Collaborative Scenario Test ===\n");

    // Connect as "agent"
    let config = get_config();
    let mut agent_session = timeout(TIMEOUT, NotebookSession::connect(config))
        .await
        .expect("Connection timed out")
        .expect("Failed to connect");

    let initial_count = agent_session.cell_count();
    println!("Agent connected. Initial cell count: {}", initial_count);

    // Agent adds a cell
    let cell_id = uuid::Uuid::new_v4().to_string();
    agent_session
        .doc()
        .add_cell(
            &cell_id,
            cell_types::CODE,
            "# Agent added this cell\nresult = 2 + 2\nprint(f'result = {result}')",
            None,
        )
        .expect("Failed to add cell");
    agent_session
        .sync_to_server()
        .await
        .expect("Failed to sync");

    println!(
        "Agent added cell. New count: {}",
        agent_session.cell_count()
    );

    // Wait for sync to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("At this point, JupyterLab should show the new cell!");

    // Agent executes the cell
    agent_session
        .connect_kernel(None)
        .await
        .expect("Failed to connect kernel");

    let last_cell = agent_session.cell_count() - 1;
    let events = timeout(
        Duration::from_secs(30),
        agent_session.execute_cell(last_cell),
    )
    .await
    .expect("Execution timed out")
    .expect("Failed to execute");

    println!("\nExecution events for cell {}:", last_cell);
    for event in &events {
        match event {
            ExecutionEvent::Started { cell_index, .. } => {
                println!("  [{}] Started", cell_index)
            }
            ExecutionEvent::ExecutionCountUpdated { cell_index, count } => {
                println!("  [{}] In [{}]", cell_index, count)
            }
            ExecutionEvent::Completed { cell_index, .. } => {
                println!("  [{}] Completed", cell_index)
            }
            ExecutionEvent::Error {
                cell_index,
                ename,
                evalue,
                ..
            } => println!("  [{}] Error: {} - {}", cell_index, ename, evalue),
            _ => {}
        }
    }

    // Clean up: remove the test cell
    agent_session
        .doc()
        .remove_cell(last_cell)
        .expect("Failed to remove cell");
    agent_session
        .sync_to_server()
        .await
        .expect("Failed to sync");
    println!("\nCleaned up test cell");

    agent_session.close().await.expect("Failed to close");
    println!("\n=== Test Complete ===");
}
