//! Mapping between kernel messages and nbformat outputs.
//!
//! This module provides conversion functions to transform kernel execution
//! output messages into nbformat Output types for storage in notebooks.

use jupyter_protocol::{
    DisplayData as KernelDisplayData, ErrorOutput as KernelErrorOutput,
    ExecuteResult as KernelExecuteResult, JupyterMessageContent, Stdio, StreamContent,
};
use nbformat::v4::{
    DisplayData as NbDisplayData, ErrorOutput as NbErrorOutput, ExecuteResult as NbExecuteResult,
    MultilineString, Output,
};

/// Represents a kernel output that should be stored in a notebook.
#[derive(Debug, Clone)]
pub enum KernelOutput {
    /// An output to append to the cell's outputs array.
    Output(Output),
    /// Signal to clear all outputs from the cell.
    /// If `wait` is true, outputs should be cleared just before the next output.
    ClearOutput { wait: bool },
}

/// Convert a kernel message content to a kernel output.
///
/// Returns `Some(KernelOutput)` if the message is an output-related message,
/// `None` otherwise.
pub fn message_to_kernel_output(content: &JupyterMessageContent) -> Option<KernelOutput> {
    match content {
        JupyterMessageContent::StreamContent(stream) => {
            Some(KernelOutput::Output(stream_to_output(stream)))
        }
        JupyterMessageContent::DisplayData(display) => {
            Some(KernelOutput::Output(display_data_to_output(display)))
        }
        JupyterMessageContent::ExecuteResult(result) => {
            Some(KernelOutput::Output(execute_result_to_output(result)))
        }
        JupyterMessageContent::ErrorOutput(error) => {
            Some(KernelOutput::Output(error_to_output(error)))
        }
        JupyterMessageContent::ClearOutput(clear) => {
            Some(KernelOutput::ClearOutput { wait: clear.wait })
        }
        _ => None,
    }
}

/// Convert a StreamContent message to an nbformat Output.
pub fn stream_to_output(stream: &StreamContent) -> Output {
    Output::Stream {
        name: match stream.name {
            Stdio::Stdout => "stdout".to_string(),
            Stdio::Stderr => "stderr".to_string(),
        },
        text: MultilineString(stream.text.clone()),
    }
}

/// Convert a kernel DisplayData message to an nbformat Output.
pub fn display_data_to_output(display: &KernelDisplayData) -> Output {
    Output::DisplayData(NbDisplayData {
        data: display.data.clone(),
        metadata: display.metadata.clone(),
    })
}

/// Convert a kernel ExecuteResult message to an nbformat Output.
pub fn execute_result_to_output(result: &KernelExecuteResult) -> Output {
    Output::ExecuteResult(NbExecuteResult {
        execution_count: result.execution_count,
        data: result.data.clone(),
        metadata: result.metadata.clone(),
    })
}

/// Convert a kernel ErrorOutput message to an nbformat Output.
pub fn error_to_output(error: &KernelErrorOutput) -> Output {
    Output::Error(NbErrorOutput {
        ename: error.ename.clone(),
        evalue: error.evalue.clone(),
        traceback: error.traceback.clone(),
    })
}

/// Helper to check if a kernel message is an output-related message.
pub fn is_output_message(content: &JupyterMessageContent) -> bool {
    matches!(
        content,
        JupyterMessageContent::StreamContent(_)
            | JupyterMessageContent::DisplayData(_)
            | JupyterMessageContent::ExecuteResult(_)
            | JupyterMessageContent::ErrorOutput(_)
            | JupyterMessageContent::ClearOutput(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use jupyter_protocol::media::{Media, MediaType};
    use jupyter_protocol::ClearOutput;

    #[test]
    fn test_stream_to_output_stdout() {
        let stream = StreamContent {
            name: Stdio::Stdout,
            text: "hello world\n".to_string(),
        };

        let output = stream_to_output(&stream);

        match output {
            Output::Stream { name, text } => {
                assert_eq!(name, "stdout");
                assert_eq!(text.0, "hello world\n");
            }
            _ => panic!("Expected Stream output"),
        }
    }

    #[test]
    fn test_stream_to_output_stderr() {
        let stream = StreamContent {
            name: Stdio::Stderr,
            text: "error message\n".to_string(),
        };

        let output = stream_to_output(&stream);

        match output {
            Output::Stream { name, text } => {
                assert_eq!(name, "stderr");
                assert_eq!(text.0, "error message\n");
            }
            _ => panic!("Expected Stream output"),
        }
    }

    #[test]
    fn test_error_to_output() {
        let error = KernelErrorOutput {
            ename: "ValueError".to_string(),
            evalue: "invalid input".to_string(),
            traceback: vec![
                "Traceback (most recent call last):".to_string(),
                "  File \"<stdin>\", line 1, in <module>".to_string(),
                "ValueError: invalid input".to_string(),
            ],
        };

        let output = error_to_output(&error);

        match output {
            Output::Error(err) => {
                assert_eq!(err.ename, "ValueError");
                assert_eq!(err.evalue, "invalid input");
                assert_eq!(err.traceback.len(), 3);
            }
            _ => panic!("Expected Error output"),
        }
    }

    #[test]
    fn test_display_data_to_output() {
        let display = KernelDisplayData {
            data: Media::new(vec![MediaType::Plain("hello".to_string())]),
            metadata: serde_json::Map::new(),
            transient: None,
        };

        let output = display_data_to_output(&display);

        match output {
            Output::DisplayData(data) => {
                assert!(!data.data.content.is_empty());
            }
            _ => panic!("Expected DisplayData output"),
        }
    }

    #[test]
    fn test_execute_result_to_output() {
        use jupyter_protocol::ExecutionCount;

        let result = KernelExecuteResult {
            execution_count: ExecutionCount::new(42),
            data: Media::new(vec![MediaType::Plain("42".to_string())]),
            metadata: serde_json::Map::new(),
            transient: None,
        };

        let output = execute_result_to_output(&result);

        match output {
            Output::ExecuteResult(res) => {
                assert_eq!(res.execution_count, ExecutionCount::new(42));
            }
            _ => panic!("Expected ExecuteResult output"),
        }
    }

    #[test]
    fn test_message_to_kernel_output_stream() {
        let content = JupyterMessageContent::StreamContent(StreamContent {
            name: Stdio::Stdout,
            text: "test".to_string(),
        });

        let output = message_to_kernel_output(&content);
        assert!(matches!(
            output,
            Some(KernelOutput::Output(Output::Stream { .. }))
        ));
    }

    #[test]
    fn test_message_to_kernel_output_clear() {
        let content = JupyterMessageContent::ClearOutput(ClearOutput { wait: true });

        let output = message_to_kernel_output(&content);
        assert!(matches!(
            output,
            Some(KernelOutput::ClearOutput { wait: true })
        ));
    }

    #[test]
    fn test_message_to_kernel_output_non_output() {
        let content = JupyterMessageContent::Status(jupyter_protocol::Status {
            execution_state: jupyter_protocol::ExecutionState::Idle,
        });

        let output = message_to_kernel_output(&content);
        assert!(output.is_none());
    }

    #[test]
    fn test_is_output_message() {
        assert!(is_output_message(&JupyterMessageContent::StreamContent(
            StreamContent::default()
        )));
        assert!(is_output_message(&JupyterMessageContent::ClearOutput(
            ClearOutput { wait: false }
        )));
        assert!(!is_output_message(&JupyterMessageContent::Status(
            jupyter_protocol::Status {
                execution_state: jupyter_protocol::ExecutionState::Idle,
            }
        )));
    }
}
