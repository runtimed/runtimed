//! Simple notebook CLI for testing Y-sync
//!
//! Usage:
//!   cargo run -p jupyter-ysync --features client --example nb -- <command> [args]
//!
//! Commands:
//!   list                     - List all cells
//!   edit <index> <source>    - Edit a cell's source
//!   add <source>             - Add a new cell
//!   status                   - Show connection status

use jupyter_ysync::{NotebookSession, SessionConfig};
use yrs::{Array, GetString, Map, ReadTxn, Text, Transact};

const BASE_URL: &str = "http://localhost:18889";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "agent_test.ipynb";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match cmd {
        "status" => {
            println!("Checking server...");
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/api/status?token={}", BASE_URL, TOKEN))
                .send()
                .await?
                .text()
                .await?;
            println!("{}", resp);
            return Ok(());
        }
        _ => {}
    }

    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH).with_token(TOKEN);
    let mut session = NotebookSession::connect(config).await?;

    match cmd {
        "list" => {
            println!("Cells: {}\n", session.cell_count());
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
                    let preview = if source.len() > 50 {
                        format!("{}...", &source[..50])
                    } else {
                        source
                    };
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
                    println!("Cell {} updated", index);
                }
            }
        }
        "add" => {
            let source = args.get(2).map(|s| s.as_str()).unwrap_or("# new cell");
            let cell_id = uuid::Uuid::new_v4().to_string();
            session
                .doc()
                .add_cell(&cell_id, jupyter_ysync::cell_types::CODE, source, None)?;
            session.sync_to_server().await?;
            println!("Added cell: {}", source.replace('\n', "\\n"));
        }
        _ => {
            println!("Unknown command: {}", cmd);
        }
    }

    session.close().await?;
    Ok(())
}
