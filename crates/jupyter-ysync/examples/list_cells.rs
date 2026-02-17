//! List all cells in a notebook.
//!
//! Usage:
//!   cargo run -p jupyter-ysync --features client --example list_cells

use jupyter_ysync::{NotebookSession, SessionConfig};
use yrs::{Array, GetString, Map, ReadTxn, Transact};

const BASE_URL: &str = "http://localhost:18889";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "agent_test.ipynb";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH).with_token(TOKEN);
    let session = NotebookSession::connect(config).await?;

    println!("Notebook: {}", NOTEBOOK_PATH);
    println!("Cell count: {}\n", session.cell_count());

    // Scope the doc access to avoid borrow conflicts
    {
        let doc = session.doc().doc();
        let txn = doc.transact();
        let cells = txn.get_array("cells").unwrap();

        for i in 0..cells.len(&txn) {
            if let Some(yrs::Out::YMap(cell_map)) = cells.get(&txn, i) {
                let cell_type = cell_map
                    .get(&txn, "cell_type")
                    .and_then(|v| match v {
                        yrs::Out::Any(yrs::Any::String(s)) => Some(s.to_string()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let source = match cell_map.get(&txn, "source") {
                    Some(yrs::Out::YText(text)) => text.get_string(&txn),
                    Some(yrs::Out::Any(yrs::Any::String(s))) => s.to_string(),
                    _ => "<no source>".to_string(),
                };

                // Truncate long sources
                let source_preview = if source.len() > 60 {
                    format!("{}...", &source[..60])
                } else {
                    source.clone()
                };

                println!("Cell {}: [{}]", i, cell_type);
                println!("  Source: {}", source_preview.replace('\n', "\\n"));
                println!();
            }
        }
    }

    session.close().await?;
    Ok(())
}
