//! Edit an existing cell in a notebook.
//!
//! This example modifies an existing cell's source (which is already Y.Text)
//! rather than adding a new cell, to verify the Y-sync update protocol works.
//!
//! Usage:
//!   1. Create a notebook in JupyterLab with at least one cell
//!   2. Run: cargo run -p jupyter-ysync --features client --example edit_cell

use jupyter_ysync::{NotebookSession, SessionConfig};
use yrs::{Array, GetString, Map, ReadTxn, Text, Transact};

const BASE_URL: &str = "http://localhost:18889";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "agent_test.ipynb";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to notebook: {}", NOTEBOOK_PATH);

    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH).with_token(TOKEN);
    let mut session = NotebookSession::connect(config).await?;

    println!("Connected! Cell count: {}", session.cell_count());

    if session.cell_count() == 0 {
        println!("No cells in notebook. Please create at least one cell in JupyterLab first.");
        session.close().await?;
        return Ok(());
    }

    // Edit the first cell's source (scoped to avoid borrow issues)
    {
        let doc = session.doc().doc();
        let txn = doc.transact();

        // Get the cells array directly to access the raw source
        let cells = txn.get_array("cells").unwrap();
        if let Some(yrs::Out::YMap(cell_map)) = cells.get(&txn, 0) {
            match cell_map.get(&txn, "source") {
                Some(yrs::Out::YText(text)) => {
                    let old_source = text.get_string(&txn);
                    println!("Source is Y.Text: {}", old_source);

                    // Modify the Y.Text
                    drop(txn);
                    let mut txn = doc.transact_mut();
                    let len = text.len(&txn);
                    text.remove_range(&mut txn, 0, len);
                    text.insert(&mut txn, 0, "# LIVE EDIT! 🦀\nprint('Hello from Rust Y-sync client!')\nprint('The time is:', __import__('datetime').datetime.now())");
                    drop(txn);

                    println!("Modified cell source");
                }
                Some(yrs::Out::Any(yrs::Any::String(s))) => {
                    println!("Source is plain string: {}", s);
                    println!("This shouldn't happen for cells from the server!");
                }
                other => {
                    println!("Source is unexpected type: {:?}", other);
                }
            }
        } else {
            println!("Could not get cell 0");
        }
    }

    // Sync the change to the server
    session.sync_to_server().await?;
    println!("Synced to server!");

    // Wait briefly for the update to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("Done! Check JupyterLab - cell source should have changed.");

    session.close().await?;
    Ok(())
}
