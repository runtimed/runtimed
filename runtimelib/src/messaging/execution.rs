use crate::messaging::{Header, JupyterMessage, JupyterMessageContent};

#[derive(Debug, Clone, serde:: Serialize, serde::Deserialize)]
pub struct CodeExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    // TODO: this could be a map from content type to data. Right
    // now we only support text/plain
    pub data: String,
    pub header: Header,
    pub start_time: String,
    pub end_time: String,
}

impl CodeExecutionOutput {
    pub fn new(header: Header) -> Self {
        Self {
            stdout: "".to_string(),
            stderr: "".to_string(),
            data: "".to_string(),
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
                self.data = execute_result.data["text/plain"].to_string();
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
            "CodeExecutionOutput\nexecution id: {}\nstart_time: {}\nend_time: {}\nstdout: {}\nstderr: {}\ndata: {}, ",
            self.header.msg_id, self.start_time, self.end_time, self.stdout, self.stderr, self.data
        )
    }
}
