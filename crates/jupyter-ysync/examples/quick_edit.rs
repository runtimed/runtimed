//! Quick edit - modify a specific cell
use jupyter_ysync::{NotebookSession, SessionConfig};
use yrs::{Array, GetString, Map, ReadTxn, Text, Transact};

const BASE_URL: &str = "http://localhost:18889";
const TOKEN: &str = "testtoken123";
const NOTEBOOK_PATH: &str = "agent_test.ipynb";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cell_index: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let new_source = args.get(2).map(|s| s.as_str()).unwrap_or("# Edited by Rust!");

    let config = SessionConfig::new(BASE_URL, NOTEBOOK_PATH).with_token(TOKEN);
    let mut session = NotebookSession::connect(config).await?;

    println!("Editing cell {}...", cell_index);

    {
        let doc = session.doc().doc();
        let txn = doc.transact();
        let cells = txn.get_array("cells").unwrap();

        if let Some(yrs::Out::YMap(cell_map)) = cells.get(&txn, cell_index) {
            if let Some(yrs::Out::YText(text)) = cell_map.get(&txn, "source") {
                drop(txn);
                let mut txn = doc.transact_mut();
                let len = text.len(&txn);
                text.remove_range(&mut txn, 0, len);
                text.insert(&mut txn, 0, new_source);
            }
        }
    }

    session.sync_to_server().await?;
    println!("Done! Cell {} updated.", cell_index);
    session.close().await?;
    Ok(())
}
