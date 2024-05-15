use runtimelib::messaging::{ErrorOutput, Header, JupyterMessage, JupyterMessageContent};
use std::collections::HashMap;

#[derive(Debug, Clone, serde:: Serialize, serde::Deserialize)]
pub struct CodeExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub result: HashMap<String, String>,
    pub error: Option<ErrorOutput>,
    pub header: Header,
    pub start_time: String,
    pub end_time: String,
}

impl CodeExecutionOutput {
    pub fn new(header: Header) -> Self {
        Self {
            stdout: "".to_string(),
            stderr: "".to_string(),
            result: HashMap::new(),
            error: None,
            header,
            start_time: "".to_string(),
            end_time: "".to_string(),
        }
    }

    pub fn add_message(&mut self, message: JupyterMessage) {
        match message.content {
            JupyterMessageContent::Status(status) => {
                if status.execution_state == "busy" {
                    self.start_time = message.header.date.to_string();
                } else if status.execution_state == "idle" {
                    self.end_time = message.header.date.to_string();
                }
            }
            JupyterMessageContent::StreamContent(stream_content) => {
                if stream_content.name == "stdout" {
                    self.stdout.push_str(&stream_content.text);
                } else if stream_content.name == "stderr" {
                    self.stderr.push_str(&stream_content.text);
                }
            }
            JupyterMessageContent::ExecuteResult(execute_result) => {
                self.result = execute_result.data.clone();
            }
            JupyterMessageContent::ErrorOutput(error) => {
                self.error = Some(error);
            }
            _ => {}
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.end_time.is_empty()
    }
}

impl std::fmt::Display for CodeExecutionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CodeExecutionOutput\nexecution id: {}\nstart_time: {}\nend_time: {}\nstdout: {}\nstderr: {}\nresult: {}",
            self.header.msg_id, self.start_time, self.end_time, self.stdout, self.stderr,
            self.result.get("text/plain").unwrap_or(&"".to_string()),
        )?;
        match &self.error {
            Some(e) => write!(f, "\nerror: {:#?}", e),
            _ => Ok(()),
        }
    }
}
