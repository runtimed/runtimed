use tokio::sync::Mutex;

use crate::jupyter::handlers::Handler;
use crate::jupyter::response::Response;
use crate::jupyter::notebook::{Notebook, Output};

use std::fmt::Debug;
use std::sync::Arc;

// Update a document model with outputs while running a cell
#[derive(Debug)]
pub struct OutputHandler {
    nb: Arc<Mutex<Notebook>>,
    cell_id: String,
    clear_on_next_output: bool,
}

impl OutputHandler {
    pub fn new(nb: Arc<Mutex<Notebook>>, cell_id: &str) -> Self {
        Self {
            nb,
            cell_id: cell_id.to_string(),
            clear_on_next_output: false,
        }
    }

    pub async fn add_output(&mut self, content: Output) {
        let mut nb = self.nb.lock().await;
        if let Some(cell) = nb.get_mut_cell(&self.cell_id) {
            cell.add_output(content);
        }
    }

    pub async fn clear_output(&mut self) {
        let mut nb = self.nb.lock().await;
        if let Some(cell) = nb.get_mut_cell(&self.cell_id) {
            cell.clear_output();
        }
    }
}

#[async_trait::async_trait]
impl Handler for OutputHandler {
    async fn handle(&mut self, msg: &Response) {
        match msg {
            Response::ExecuteResult(m) => {
                let output = Output::ExecuteResult(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::Stream(m) => {
                let output = Output::Stream(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::DisplayData(m) => {
                let output = Output::DisplayData(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::Error(m) => {
                let output = Output::Error(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::ClearOutput(m) => {
                if m.content.wait {
                    self.clear_on_next_output = true;
                } else {
                    self.clear_output().await;
                }
            }
            _ => {}
        }
    }
}

// SimpleOutputHandler doesn't update a document model, just stores a list of outputs in memory.
// Useful for tests and maybe debug / demos, probably not something you care about for app building
#[derive(Debug, Clone)]
pub struct SimpleOutputHandler {
    // interior mutability here because .handle needs to set this and is &self, and when trying
    // to change that to &mut self then it broke the delegation of ZMQ messages to Actions over
    // in actions.rs. TODO: come back to this when I'm better at Rust?
    clear_on_next_output: bool,
    pub output: Vec<Output>,
}

impl Default for SimpleOutputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleOutputHandler {
    pub fn new() -> Self {
        Self {
            clear_on_next_output: false,
            output: vec![],
        }
    }

    async fn add_output(&mut self, content: Output) {
        self.output.push(content);
        println!("adding output");
    }

    async fn clear_output(&mut self) {
        self.output.clear();
        println!("clearing output");
    }
}

#[async_trait::async_trait]
impl Handler for SimpleOutputHandler {
    async fn handle(&mut self, msg: &Response) {
        match msg {
            Response::ExecuteResult(m) => {
                let output = Output::ExecuteResult(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::Stream(m) => {
                let output = Output::Stream(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::DisplayData(m) => {
                let output = Output::DisplayData(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::Error(m) => {
                let output = Output::Error(m.content.clone());
                if self.clear_on_next_output {
                    self.clear_output().await;
                    self.clear_on_next_output = false;
                }
                self.add_output(output).await;
            }
            Response::ClearOutput(m) => {
                if m.content.wait {
                    self.clear_on_next_output = true;
                } else {
                    self.clear_output().await;
                }
            }
            _ => {}
        }
    }
}
