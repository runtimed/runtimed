//! Execution coordination for notebook cells.
//!
//! This module provides the `CellExecutor` which tracks cell executions
//! and routes kernel outputs to the correct cell in a Y.Doc notebook.

use std::collections::HashMap;

use jupyter_protocol::{
    ExecuteReply, ExecutionState, JupyterMessage, JupyterMessageContent, ReplyStatus,
};

use crate::doc::NotebookDoc;
use crate::error::Result;
use crate::output_mapping::{message_to_kernel_output, KernelOutput};

/// Events emitted during cell execution.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Execution has started (kernel is busy).
    Started {
        cell_index: u32,
        msg_id: String,
    },
    /// An output was added to the cell.
    OutputAdded {
        cell_index: u32,
    },
    /// Outputs were cleared.
    OutputsCleared {
        cell_index: u32,
    },
    /// Execution count was updated.
    ExecutionCountUpdated {
        cell_index: u32,
        count: i32,
    },
    /// Execution completed successfully.
    Completed {
        cell_index: u32,
        msg_id: String,
    },
    /// Execution failed with an error.
    Error {
        cell_index: u32,
        msg_id: String,
        ename: String,
        evalue: String,
    },
}

/// Tracks state for a pending cell execution.
#[derive(Debug)]
struct PendingExecution {
    /// The cell index being executed.
    cell_index: u32,
    /// Whether we've seen the busy status.
    saw_busy: bool,
    /// Whether clear_output with wait=true is pending.
    pending_clear: bool,
}

/// Coordinates cell execution and routes kernel outputs to the notebook.
///
/// The executor tracks pending executions by their parent message ID
/// and updates the notebook document when outputs are received.
#[derive(Debug)]
pub struct CellExecutor {
    /// Map from message ID to pending execution info.
    pending: HashMap<String, PendingExecution>,
}

impl CellExecutor {
    /// Create a new cell executor.
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Register a new execution request.
    ///
    /// Call this when sending an execute_request to the kernel.
    /// The `msg_id` should be the message ID from the request header.
    pub fn register_execution(&mut self, msg_id: String, cell_index: u32) {
        self.pending.insert(
            msg_id,
            PendingExecution {
                cell_index,
                saw_busy: false,
                pending_clear: false,
            },
        );
    }

    /// Check if there are any pending executions.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get the number of pending executions.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Handle a kernel message and update the notebook.
    ///
    /// Returns a list of execution events that occurred, or None if the
    /// message was not related to any pending execution.
    pub fn handle_message(
        &mut self,
        msg: &JupyterMessage,
        doc: &NotebookDoc,
    ) -> Result<Option<Vec<ExecutionEvent>>> {
        // Get the parent message ID - if none, not related to an execution
        let parent_msg_id = match &msg.parent_header {
            Some(header) => &header.msg_id,
            None => return Ok(None),
        };

        // Check if this is for a pending execution
        let pending = match self.pending.get_mut(parent_msg_id) {
            Some(p) => p,
            None => return Ok(None),
        };

        let cell_index = pending.cell_index;
        let mut events = Vec::new();

        match &msg.content {
            // Status messages track execution lifecycle
            JupyterMessageContent::Status(status) => {
                match status.execution_state {
                    ExecutionState::Busy => {
                        pending.saw_busy = true;
                        events.push(ExecutionEvent::Started {
                            cell_index,
                            msg_id: parent_msg_id.clone(),
                        });
                    }
                    ExecutionState::Idle if pending.saw_busy => {
                        // Execution is complete - remove from pending
                        self.pending.remove(parent_msg_id);
                        events.push(ExecutionEvent::Completed {
                            cell_index,
                            msg_id: parent_msg_id.clone(),
                        });
                    }
                    _ => {}
                }
            }

            // Execute reply gives us execution count and status
            JupyterMessageContent::ExecuteReply(reply) => {
                if let Some(count) = execution_count_from_reply(reply) {
                    doc.set_execution_count(cell_index, Some(count))?;
                    events.push(ExecutionEvent::ExecutionCountUpdated { cell_index, count });
                }

                // Check for error status
                if reply.status == ReplyStatus::Error {
                    events.push(ExecutionEvent::Error {
                        cell_index,
                        msg_id: parent_msg_id.clone(),
                        ename: reply
                            .error
                            .as_ref()
                            .map(|e| e.ename.clone())
                            .unwrap_or_default(),
                        evalue: reply
                            .error
                            .as_ref()
                            .map(|e| e.evalue.clone())
                            .unwrap_or_default(),
                    });
                }
            }

            // Output messages get added to the cell
            content => {
                if let Some(kernel_output) = message_to_kernel_output(content) {
                    match kernel_output {
                        KernelOutput::Output(output) => {
                            // If there's a pending clear, execute it first
                            if pending.pending_clear {
                                doc.clear_cell_outputs(cell_index)?;
                                pending.pending_clear = false;
                                events.push(ExecutionEvent::OutputsCleared { cell_index });
                            }

                            doc.append_output(cell_index, &output)?;
                            events.push(ExecutionEvent::OutputAdded { cell_index });
                        }
                        KernelOutput::ClearOutput { wait } => {
                            if wait {
                                // Clear before next output
                                pending.pending_clear = true;
                            } else {
                                // Clear immediately
                                doc.clear_cell_outputs(cell_index)?;
                                events.push(ExecutionEvent::OutputsCleared { cell_index });
                            }
                        }
                    }
                }
            }
        }

        if events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(events))
        }
    }

    /// Cancel a pending execution.
    ///
    /// Returns true if the execution was found and removed.
    pub fn cancel(&mut self, msg_id: &str) -> bool {
        self.pending.remove(msg_id).is_some()
    }

    /// Cancel all pending executions.
    pub fn cancel_all(&mut self) {
        self.pending.clear();
    }

    /// Get the cell index for a pending execution.
    pub fn get_cell_index(&self, msg_id: &str) -> Option<u32> {
        self.pending.get(msg_id).map(|p| p.cell_index)
    }
}

impl Default for CellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract execution count from an execute reply.
fn execution_count_from_reply(reply: &ExecuteReply) -> Option<i32> {
    let count = reply.execution_count.value();
    // 0 typically means not set/unknown
    if count > 0 {
        Some(count as i32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::cell_types;
    use jupyter_protocol::{
        ExecuteReply, ExecuteRequest, ExecutionCount, ReplyStatus, Status, Stdio, StreamContent,
    };
    use yrs::{Array, Transact};

    /// Create a child message with a specific parent message ID.
    fn make_child_message(content: JupyterMessageContent, parent: &JupyterMessage) -> JupyterMessage {
        JupyterMessage::new(content, Some(parent))
    }

    /// Create a parent execute request message with a specific message ID.
    fn make_execute_request(msg_id: &str) -> JupyterMessage {
        let mut msg = JupyterMessage::new(
            ExecuteRequest {
                code: "print('test')".to_string(),
                silent: false,
                store_history: true,
                user_expressions: Default::default(),
                allow_stdin: false,
                stop_on_error: true,
            },
            None,
        );
        // Override the generated msg_id with our test ID
        msg.header.msg_id = msg_id.to_string();
        msg
    }

    #[test]
    fn test_register_execution() {
        let mut executor = CellExecutor::new();
        assert!(!executor.has_pending());

        executor.register_execution("msg-1".to_string(), 0);
        assert!(executor.has_pending());
        assert_eq!(executor.pending_count(), 1);
        assert_eq!(executor.get_cell_index("msg-1"), Some(0));
    }

    #[test]
    fn test_handle_status_busy() {
        let mut executor = CellExecutor::new();
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();

        let parent = make_execute_request("msg-1");
        executor.register_execution("msg-1".to_string(), 0);

        let msg = make_child_message(
            JupyterMessageContent::Status(Status {
                execution_state: ExecutionState::Busy,
            }),
            &parent,
        );

        let events = executor.handle_message(&msg, &doc).unwrap();
        assert!(events.is_some());
        let events = events.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ExecutionEvent::Started { cell_index: 0, .. }));
    }

    #[test]
    fn test_handle_stream_output() {
        let mut executor = CellExecutor::new();
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();

        let parent = make_execute_request("msg-1");
        executor.register_execution("msg-1".to_string(), 0);

        // Mark as busy first
        let busy_msg = make_child_message(
            JupyterMessageContent::Status(Status {
                execution_state: ExecutionState::Busy,
            }),
            &parent,
        );
        executor.handle_message(&busy_msg, &doc).unwrap();

        // Handle stream output
        let msg = make_child_message(
            JupyterMessageContent::StreamContent(StreamContent {
                name: Stdio::Stdout,
                text: "hello\n".to_string(),
            }),
            &parent,
        );

        let events = executor.handle_message(&msg, &doc).unwrap();
        assert!(events.is_some());
        let events = events.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ExecutionEvent::OutputAdded { cell_index: 0 }));

        // Verify output was added to cell
        let cell = doc.get_cell(0).unwrap();
        let txn = doc.doc().transact();
        let outputs = cell.outputs(&txn).unwrap();
        assert_eq!(outputs.len(&txn), 1);
    }

    #[test]
    fn test_execution_lifecycle() {
        let mut executor = CellExecutor::new();
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "print('hello')", None)
            .unwrap();

        let parent = make_execute_request("msg-1");
        executor.register_execution("msg-1".to_string(), 0);
        assert!(executor.has_pending());

        // Busy
        let busy = make_child_message(
            JupyterMessageContent::Status(Status {
                execution_state: ExecutionState::Busy,
            }),
            &parent,
        );
        let events = executor.handle_message(&busy, &doc).unwrap().unwrap();
        assert!(matches!(events[0], ExecutionEvent::Started { .. }));

        // Execute reply
        let reply = make_child_message(
            JupyterMessageContent::ExecuteReply(ExecuteReply {
                status: ReplyStatus::Ok,
                execution_count: ExecutionCount::new(1),
                error: None,
                payload: Vec::new(),
                user_expressions: None,
            }),
            &parent,
        );
        let events = executor.handle_message(&reply, &doc).unwrap().unwrap();
        assert!(matches!(
            events[0],
            ExecutionEvent::ExecutionCountUpdated { cell_index: 0, count: 1 }
        ));

        // Verify execution count was set
        let cell = doc.get_cell(0).unwrap();
        let txn = doc.doc().transact();
        assert_eq!(cell.execution_count(&txn), Some(Some(1)));

        // Idle - completes execution
        let idle = make_child_message(
            JupyterMessageContent::Status(Status {
                execution_state: ExecutionState::Idle,
            }),
            &parent,
        );
        let events = executor.handle_message(&idle, &doc).unwrap().unwrap();
        assert!(matches!(events[0], ExecutionEvent::Completed { .. }));
        assert!(!executor.has_pending());
    }

    #[test]
    fn test_cancel_execution() {
        let mut executor = CellExecutor::new();
        executor.register_execution("msg-1".to_string(), 0);
        executor.register_execution("msg-2".to_string(), 1);
        assert_eq!(executor.pending_count(), 2);

        assert!(executor.cancel("msg-1"));
        assert_eq!(executor.pending_count(), 1);

        assert!(!executor.cancel("msg-1")); // Already cancelled
        assert!(executor.cancel("msg-2"));
        assert!(!executor.has_pending());
    }

    #[test]
    fn test_unrelated_message_ignored() {
        let mut executor = CellExecutor::new();
        let doc = NotebookDoc::new();
        doc.add_cell("cell-1", cell_types::CODE, "x = 1", None)
            .unwrap();

        let parent_other = make_execute_request("msg-other");
        executor.register_execution("msg-1".to_string(), 0);

        // Message with different parent ID
        let msg = make_child_message(
            JupyterMessageContent::StreamContent(StreamContent {
                name: Stdio::Stdout,
                text: "hello\n".to_string(),
            }),
            &parent_other,
        );

        let events = executor.handle_message(&msg, &doc).unwrap();
        assert!(events.is_none());
    }
}
