//! Notebook CLI for testing Y-sync collaborative editing.
//!
//! This is a simple CLI tool for interacting with Jupyter notebooks via the
//! Y-sync protocol (jupyter-server-documents). **BETA** - for testing only.
//!
//! # Prerequisites
//!
//! 1. Start a Jupyter server with jupyter-server-documents:
//!    ```bash
//!    uv run --with jupyter-server --with jupyter-server-documents --with jupyterlab \
//!      jupyter lab --port 18889 --IdentityProvider.token=testtoken123
//!    ```
//!
//! 2. Open a notebook in JupyterLab (the collaboration room must be active)
//!
//! # Usage
//!
//! ```bash
//! cargo run -p jupyter-ysync --features client --example nb -- <command> [args]
//! ```
//!
//! # Commands
//!
//! - `status` - Check server status (doesn't require notebook open)
//! - `sessions` - List active sessions (to check if JupyterLab has a kernel)
//! - `list` - List all cells in the notebook
//! - `edit <index> <source>` - Edit a cell's source code
//! - `add <source>` - Add a new code cell
//! - `execute <index>` - Execute a cell (alias: `exec`)
//! - `run <indices>` - Execute multiple cells, keeping kernel alive (e.g., `run 0,1,2`)
//!
//! # Environment Variables
//!
//! - `JUPYTER_URL` - Base URL (default: http://localhost:18889)
//! - `JUPYTER_TOKEN` - Auth token (default: testtoken123)
//! - `NOTEBOOK_PATH` - Notebook path (default: test.ipynb)
//!
//! # Examples
//!
//! ```bash
//! # Check server status
//! cargo run -p jupyter-ysync --features client --example nb -- status
//!
//! # List cells
//! cargo run -p jupyter-ysync --features client --example nb -- list
//!
//! # Edit cell 0
//! cargo run -p jupyter-ysync --features client --example nb -- edit 0 "print('hello')"
//!
//! # Add a new cell
//! cargo run -p jupyter-ysync --features client --example nb -- add "x = 42"
//!
//! # Execute a cell
//! cargo run -p jupyter-ysync --features client --example nb -- exec 0
//! ```
//!
//! # Kernel Lifecycle
//!
//! Jupyter Server shuts down kernels when all WebSocket connections close.
//! For persistent execution state across CLI invocations:
//!
//! 1. **Run a cell in JupyterLab first** - This creates a persistent session
//! 2. **Keep JupyterLab open** - JupyterLab's connection keeps the kernel alive
//! 3. **CLI reuses JupyterLab's session** - Variables persist between CLI calls
//!
//! If you run execution commands without JupyterLab having a session, each
//! invocation creates a fresh kernel (no variable persistence).
//!
//! # Known Issues
//!
//! - Notebook must be open in JupyterLab for Y-sync room to be active
//! - Rapid connections may cause JupyterLab frontend issues

use jupyter_ysync::{ExecutionEvent, NotebookSession, SessionConfig};
use yrs::{Array, GetString, Map, ReadTxn, Text, Transact};

fn get_config() -> (String, String, String) {
    let base_url = std::env::var("JUPYTER_URL").unwrap_or_else(|_| "http://localhost:18889".into());
    let token = std::env::var("JUPYTER_TOKEN").unwrap_or_else(|_| "testtoken123".into());
    let notebook = std::env::var("NOTEBOOK_PATH").unwrap_or_else(|_| "test.ipynb".into());
    (base_url, token, notebook)
}

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
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("list");
    let (base_url, token, notebook_path) = get_config();

    // Status command doesn't need Y-sync connection
    if cmd == "status" {
        println!("Server: {}", base_url);
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/status?token={}", base_url, token))
            .send()
            .await?
            .text()
            .await?;
        println!("{}", resp);
        return Ok(());
    }

    // Sessions command shows existing sessions (helpful for debugging kernel lifecycle)
    if cmd == "sessions" {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/sessions?token={}", base_url, token))
            .send()
            .await?
            .text()
            .await?;

        #[derive(serde::Deserialize)]
        struct Session {
            id: String,
            path: String,
            kernel: SessionKernel,
        }
        #[derive(serde::Deserialize)]
        struct SessionKernel {
            id: String,
            connections: u32,
            execution_state: String,
        }

        if let Ok(sessions) = serde_json::from_str::<Vec<Session>>(&resp) {
            if sessions.is_empty() {
                println!("No active sessions.");
                println!("\nTip: Run a cell in JupyterLab first to create a persistent session.");
            } else {
                for s in &sessions {
                    // Safely truncate kernel ID (avoid panic if < 8 chars)
                    let kernel_short: String = s.kernel.id.chars().take(8).collect();
                    println!(
                        "{}: kernel={} ({}), connections={}",
                        s.path, kernel_short, s.kernel.execution_state, s.kernel.connections
                    );
                }
                println!("\nKernels with connections > 0 will stay alive.");
            }
        } else {
            println!("{}", resp);
        }
        return Ok(());
    }

    // Connect to notebook via Y-sync
    println!("Connecting to {}...", notebook_path);
    let config = SessionConfig::new(&base_url, &notebook_path)
        .with_token(&token)
        .with_local_outputs(); // Write execution outputs to Y.Doc
    let mut session = NotebookSession::connect(config).await?;
    println!("Connected! Cells: {}\n", session.cell_count());

    match cmd {
        "list" => {
            let doc = session.doc().doc();
            let txn = doc.transact();
            let cells = txn.get_array("cells").unwrap();

            for i in 0..cells.len(&txn) {
                if let Some(yrs::Out::YMap(cell_map)) = cells.get(&txn, i) {
                    let source = match cell_map.get(&txn, "source") {
                        Some(yrs::Out::YText(text)) => text.get_string(&txn),
                        Some(yrs::Out::Any(yrs::Any::String(s))) => s.to_string(),
                        _ => "<no source>".to_string(),
                    };
                    let preview = truncate_preview(&source, 60);
                    println!("[{}] {}", i, preview.replace('\n', "\\n"));
                }
            }
        }
        "edit" => {
            let index: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            let source = args.get(3).map(|s| s.as_str()).unwrap_or("# edited");

            let doc = session.doc().doc();
            let txn = doc.transact();
            let cells = txn.get_array("cells").unwrap();

            if let Some(yrs::Out::YMap(cell_map)) = cells.get(&txn, index) {
                if let Some(yrs::Out::YText(text)) = cell_map.get(&txn, "source") {
                    drop(txn);
                    let mut txn = doc.transact_mut();
                    let len = text.len(&txn);
                    text.remove_range(&mut txn, 0, len);
                    text.insert(&mut txn, 0, source);
                    drop(txn);
                    session.sync_to_server().await?;
                    println!("Updated cell {}", index);
                }
            } else {
                println!("Cell {} not found", index);
            }
        }
        "add" => {
            let source = args.get(2).map(|s| s.as_str()).unwrap_or("# new cell");
            let cell_id = uuid::Uuid::new_v4().to_string();
            session
                .doc()
                .add_cell(&cell_id, jupyter_ysync::cell_types::CODE, source, None)?;
            session.sync_to_server().await?;
            println!("Added cell");
        }
        "execute" | "exec" => {
            let index: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

            println!("Connecting to kernel...");
            session.connect_kernel(None).await?;
            println!("Session ID: {:?}", session.session_id());
            println!("Kernel ID: {:?}", session.kernel_id());

            println!("Executing cell {}...", index);
            let events = session.execute_cell(index).await?;

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
                    } => {
                        println!("  [{}] Error: {} - {}", cell_index, ename, evalue)
                    }
                    _ => {}
                }
            }

            session.sync_to_server().await?;
            println!("Done! Check JupyterLab for outputs.");
        }
        "run" => {
            // Parse comma-separated cell indices (e.g., "0,1,2" or "0-2")
            let indices_str = args.get(2).map(|s| s.as_str()).unwrap_or("0");
            let indices: Vec<u32> = indices_str
                .split(',')
                .filter_map(|s| {
                    let s = s.trim();
                    if s.contains('-') {
                        // Handle range like "0-2"
                        let parts: Vec<&str> = s.split('-').collect();
                        if parts.len() == 2 {
                            let start: u32 = parts[0].parse().ok()?;
                            let end: u32 = parts[1].parse().ok()?;
                            return Some((start..=end).collect::<Vec<_>>());
                        }
                        None
                    } else {
                        s.parse().ok().map(|n| vec![n])
                    }
                })
                .flatten()
                .collect();

            if indices.is_empty() {
                eprintln!("No valid cell indices provided. Use: run 0,1,2 or run 0-2");
                return Ok(());
            }

            println!("Connecting to kernel...");
            session.connect_kernel(None).await?;
            println!("Session ID: {:?}", session.session_id());
            println!("Kernel ID: {:?}", session.kernel_id());
            println!();

            // Execute cells in sequence, keeping kernel connection open
            for index in &indices {
                println!("Executing cell {}...", index);
                let events = session.execute_cell(*index).await?;

                for event in &events {
                    match event {
                        ExecutionEvent::Started { .. } => {}
                        ExecutionEvent::ExecutionCountUpdated { count, .. } => {
                            println!("  In [{}]", count)
                        }
                        ExecutionEvent::Completed { .. } => {
                            println!("  Completed")
                        }
                        ExecutionEvent::Error { ename, evalue, .. } => {
                            println!("  Error: {} - {}", ename, evalue)
                        }
                        _ => {}
                    }
                }
                println!();
            }

            session.sync_to_server().await?;
            println!("All cells executed. Check JupyterLab for outputs.");
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
            eprintln!("Commands: status, sessions, list, edit, add, exec, run");
        }
    }

    // Brief delay to allow server to process pending updates before disconnect
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    session.close().await?;
    Ok(())
}
