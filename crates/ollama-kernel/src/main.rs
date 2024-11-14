use std::collections::HashMap;
#[allow(unused)]
use std::{collections::HashSet, time::Duration};

use anyhow::{Context as _, Result};

mod ollama_client;
mod structured_calling;

use structured_calling::Structured;

#[allow(unused)]
use futures::{channel::mpsc, SinkExt as _, StreamExt};
use ollama_client::{
    ChatMessage, Format, GenerateResponse, LocalModelListing, OllamaClient, Role, OLLAMA_ENDPOINT,
};
use runtimelib::{
    ClearOutput, CodeMirrorMode, CommInfoReply, CompleteReply, CompleteRequest, ConnectionInfo,
    DisplayData, ErrorOutput, ExecuteReply, ExecutionCount, HelpLink, HistoryReply, InspectReply,
    IsCompleteReply, IsCompleteReplyStatus, JupyterMessage, JupyterMessageContent, KernelInfoReply,
    KernelIoPubConnection, KernelShellConnection, LanguageInfo, Media, MediaType, ReplyStatus,
    Status, StreamContent,
};
use serde_json::Value;
use uuid::Uuid;

struct OllamaKernel {
    model: String,
    execution_count: ExecutionCount,
    iopub: KernelIoPubConnection,
    previous_messages: Vec<ChatMessage>,
    last_context: Vec<usize>,
}

/// Convert a magic cell like `%model --set gemma`
fn split_magic(input: &str) -> (&str, Option<&str>) {
    input
        .split_once('\n')
        .map(|(header, body)| (header, Some(body)))
        .unwrap_or((input, None))
}

impl OllamaKernel {
    pub async fn start(model: String, connection_info: &ConnectionInfo) -> Result<()> {
        let session_id = Uuid::new_v4().to_string();

        let mut heartbeat = runtimelib::create_kernel_heartbeat_connection(connection_info).await?;
        let shell_connection =
            runtimelib::create_kernel_shell_connection(connection_info, &session_id).await?;
        let mut control_connection =
            runtimelib::create_kernel_control_connection(connection_info, &session_id).await?;
        let _stdin_connection =
            runtimelib::create_kernel_stdin_connection(connection_info, &session_id).await?;
        let iopub_connection =
            runtimelib::create_kernel_iopub_connection(connection_info, &session_id).await?;
        // let (mut tx, rx) = futures::channel::mpsc::unbounded::<JupyterMessage>();

        let mut ollama_kernel = Self {
            model,
            execution_count: Default::default(),
            iopub: iopub_connection,
            previous_messages: Default::default(),
            last_context: Default::default(),
        };

        let heartbeat_handle = tokio::spawn({
            async move { while let Ok(()) = heartbeat.single_heartbeat().await {} }
        });

        let control_handle = tokio::spawn({
            async move {
                while let Ok(message) = control_connection.read().await {
                    if let JupyterMessageContent::KernelInfoRequest(_) = message.content {
                        let sent = control_connection
                            .send(Self::kernel_info().as_child_of(&message))
                            .await;

                        match sent {
                            Ok(_) => {}
                            Err(err) => eprintln!("Error on control {}", err),
                        }
                    }
                }
            }
        });

        let shell_handle = tokio::spawn(async move {
            if let Err(err) = ollama_kernel.handle_shell(shell_connection).await {
                eprintln!("Shell error: {}\nBacktrace:\n{}", err, err.backtrace());
            }
        });

        let join_fut =
            futures::future::try_join_all(vec![heartbeat_handle, control_handle, shell_handle]);

        join_fut.await?;

        Ok(())
    }

    async fn clear_output_after_next_output(
        &mut self,
        parent: &JupyterMessage,
    ) -> anyhow::Result<()> {
        self.iopub
            .send(ClearOutput { wait: true }.as_child_of(parent))
            .await
    }

    async fn send_markdown(
        &mut self,
        markdown: &str,
        parent: &JupyterMessage,
    ) -> anyhow::Result<()> {
        self.iopub
            .send(DisplayData::from(MediaType::Markdown(markdown.to_string())).as_child_of(parent))
            .await
    }

    async fn send_json(
        &mut self,
        json_object: Value,
        parent: &JupyterMessage,
    ) -> anyhow::Result<()> {
        let json_object = match json_object {
            Value::Object(obj) => obj,
            _ => {
                let mut map = serde_json::Map::new();
                map.insert("value".to_string(), json_object);
                map
            }
        };

        self.iopub
            .send(DisplayData::from(MediaType::Json(json_object)).as_child_of(parent))
            .await
    }

    async fn send_error(
        &mut self,
        ename: &str,
        evalue: &str,
        parent: &JupyterMessage,
    ) -> anyhow::Result<()> {
        self.iopub
            .send(
                ErrorOutput {
                    ename: ename.to_string(),
                    evalue: evalue.to_string(),
                    traceback: Default::default(),
                }
                .as_child_of(parent),
            )
            .await
    }

    async fn push_stdout(&mut self, text: &str, parent: &JupyterMessage) -> anyhow::Result<()> {
        self.iopub
            .send(StreamContent::stdout(text).as_child_of(parent))
            .await
    }

    async fn command(&mut self, command: &str, parent: &JupyterMessage) -> anyhow::Result<()> {
        let (header, body) = split_magic(command);

        let tokens: Vec<&str> = header.split_whitespace().collect();

        let mut ollama_client = OllamaClient::new();

        match tokens[..] {
            [] | ["h"] | ["help"] => {
                self.send_markdown(
                    r#"
# Model curation

* **`%model`**: Get the current model
* **`%model --list`**: List the available models
* **`%model --create {name}`**: Create or update the modelfile for {name}. The cell body below the command is the Modelfile.
* **`%model --show {name}`**: Show details for the named model. Omit name to get details on the current model.

# Conversation

* **`%use {name}`**: Set the currently used model to `{name}`
* **`%reset`**: Clear out the conversation history

# Help

* **`%help`**: call this help menu
"#
                    .trim(),
                    parent,
                )
                .await?;
            }
            ["reset"] => {
                self.previous_messages.clear();
                self.last_context.clear()
            }
            ["model", "--list"] => {
                let models = ollama_client.list_local_models().await?;

                let reformatted_models: HashMap<String, LocalModelListing> =
                    models.into_iter().map(|m| (m.name.clone(), m)).collect();

                let json_value = serde_json::to_value(reformatted_models)?;

                self.send_json(json_value, parent).await?;
            }
            ["use", name] => {
                // todo: check that it's a valid model
                self.model = name.to_string();
                let message = format!("Set model to {name}");

                self.send_markdown(&message, parent).await?;
            }
            ["model", "--create", name] => {
                let body = match body {
                    Some(body) => body,
                    None => {
                        self.send_error("Missing Modelfile Body", "", parent)
                            .await?;
                        return Ok(());
                    }
                };

                let mut updates = ollama_client.create(name, body).await?;

                while let Some(Ok(update)) = updates.next().await {
                    self.send_markdown(&update.status, parent).await?;
                    self.clear_output_after_next_output(parent).await?;
                }
                self.send_markdown("Model created", parent).await?;
            }
            ["model", "--show", ..] | ["model"] => {
                let name = match tokens[..] {
                    ["model", "--show", name] => name,
                    _ => &self.model.clone(),
                };

                let message = format!("Getting details for model: {}", name);
                self.send_markdown(&message, parent).await?;
                self.clear_output_after_next_output(parent).await?;

                let listing = ollama_client.show(name).await?;
                let mut display = String::new();

                display += "# ";
                display += name;

                display += "\n## Modelfile\n\n";
                display += "```docker\n";
                display += &listing.modelfile;
                display += "\n```\n";

                if let Some(parameters) = &listing.parameters {
                    display += "\n## Parameters\n\n";
                    display += "```\n";
                    display += parameters;
                    display += "\n```\n";
                }

                display += "\n## Template\n\n";
                display += "```\n";
                display += &listing.template;
                display += "\n```\n";

                self.send_markdown(&display, parent).await?;
                self.send_json(serde_json::to_value(listing.details)?, parent)
                    .await?;
            }
            // TODO: Delete this command, just experimenting with the API
            ["generate"] => {
                let body = match body {
                    Some(body) => body,
                    None => {
                        self.send_error("Missing Modelfile Body", "", parent)
                            .await?;
                        return Ok(());
                    }
                };

                let mut stream = ollama_client
                    .generate(
                        &self.model,
                        body.trim(),
                        &self.last_context,
                        Some(Format::Json),
                        Some(
                            r#"Only respond with JSON using the following schemas:

type Generic = {
    type: "generic";
    // Message shown to user in response
    message: string;
};

type Thought = {
    type: "thought";

    // Record your thoughts here in order to craft a better response
    chain_of_thought: string;

    // Message shown to user in response
    message: string;
};

// Use the Alert type when the user says something worrisome.
type Alert = {
  type: "alert",

  // Message shown to user in response
  message: string;

  // Report what the occurence was for someone to triage
  report: string;

  raw_user_message: string;
};

type Response = Generic | Thought | Alert;
type Responses = Response[];
                            "#
                            .trim(),
                        ),
                    )
                    .await?;

                let mut in_progress_assistant_response = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(GenerateResponse::Delta(delta)) => {
                            // self.push_stdout(&delta.response, parent).await?;
                            in_progress_assistant_response.push_str(&delta.response);
                        }
                        Ok(GenerateResponse::Finished(finished)) => {
                            self.last_context = finished.context;
                        }
                        Err(err) => {
                            self.send_error("Ollama client failure", &err.to_string(), parent)
                                .await?
                        }
                    }
                }

                self.send_json(
                    serde_json::from_str(&in_progress_assistant_response)?,
                    parent,
                )
                .await?;

                // self.clear_output_after_next_output(parent).await?;

                // self.send_markdown(&in_progress_assistant_response, parent)
                //     .await?
            }
            _ => self.send_error("Unknown command", header, parent).await?,
        };

        anyhow::Ok(())
    }

    async fn complete(&mut self, request: &CompleteRequest) -> anyhow::Result<CompleteReply> {
        let cursor_pos = request.cursor_pos;

        let mut ollama_client = OllamaClient::new();

        let (text_before, text_after) = match request.code.split_at_checked(cursor_pos) {
            Some(text) => text,
            None => {
                eprintln!("Invalid cursor position requested");
                eprintln!("{:?}", &request);
                return Err(anyhow::anyhow!("Invalid cursor position"));
            }
        };

        let system =
            r#"Only respond with JSON using the following schema for a completion response:

```typescript
{
  type: "completions";
  options: Array<string>;
}
```

The user will be sending the exact text from their notebook cell. Their cursor position is indicated with `<cursor_pos>`.

Please generate a few responses to complete their text for them.
        "#
            .trim();

        let body = format!("{}<cursor_pos>{}", text_before, text_after);

        println!("{}", &body);

        let mut stream = ollama_client
            .generate(
                &self.model,
                &body,
                &Default::default(),
                Some(Format::Json),
                Some(system),
            )
            .await?;

        let mut in_progress_assistant_response = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(GenerateResponse::Delta(delta)) => {
                    in_progress_assistant_response.push_str(&delta.response);
                }
                Ok(GenerateResponse::Finished(finally)) => {
                    self.last_context = finally.context;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }

        let matches = match serde_json::from_str::<Structured>(&in_progress_assistant_response) {
            Ok(Structured::Completions(completions)) => completions.options,
            Err(err) => {
                eprintln!("{:?}", err);
                vec![]
            }
        };

        let reply = CompleteReply {
            matches,
            cursor_start: cursor_pos,
            cursor_end: cursor_pos,
            metadata: Default::default(),
            status: runtimelib::ReplyStatus::Ok,
            error: None,
        };

        anyhow::Ok(reply)
    }

    async fn execute(&mut self, request: &JupyterMessage) -> anyhow::Result<()> {
        let code = match &request.content {
            runtimelib::JupyterMessageContent::ExecuteRequest(req) => req.code.clone(),
            _ => return Err(anyhow::anyhow!("Invalid message type for execution")),
        };

        // "Comments"
        if code.starts_with("//") {
            return Ok(());
        }

        // "Magics"
        if let Some(command) = code.strip_prefix("%") {
            return self.command(command, request).await;
        }

        self.previous_messages.push(ChatMessage {
            role: Role::User,
            content: code,
        });

        self.send_markdown("_connecting to model_", request).await?;

        // Clear the progress message after the first tokens come in
        self.clear_output_after_next_output(request).await?;

        let mut in_progress_assistant_response = String::new();

        let mut ollama_client = OllamaClient::new();
        let mut chunks = ollama_client
            .chat(&self.model, &self.previous_messages)
            .await?;

        while let Some(chunk) = chunks.next().await {
            match chunk {
                Ok(response) => {
                    let text_delta = response.message.content;

                    in_progress_assistant_response.push_str(&text_delta);

                    self.push_stdout(&text_delta, request).await?;
                }
                Err(err) => {
                    self.send_error("OllamaKernelError", &err.to_string(), request)
                        .await?;
                }
            }
        }

        if !in_progress_assistant_response.trim().is_empty() {
            self.clear_output_after_next_output(request).await?;
            self.send_markdown(&in_progress_assistant_response, request)
                .await?;

            self.previous_messages.push(ChatMessage {
                role: Role::Assistant,
                content: in_progress_assistant_response,
            });
        }

        anyhow::Ok(())
    }

    pub async fn handle_shell(&mut self, mut connection: KernelShellConnection) -> Result<()> {
        loop {
            let msg = connection.read().await?;
            match self.handle_shell_message(&msg, &mut connection).await {
                Ok(_) => {}
                Err(err) => eprintln!("Error on shell: {}", err),
            }
        }
    }

    pub async fn handle_shell_message(
        &mut self,
        parent: &JupyterMessage,
        shell: &mut KernelShellConnection,
    ) -> Result<()> {
        // Even with messages like `kernel_info_request`, you're required to send a busy and idle message
        self.iopub.send(Status::busy().as_child_of(parent)).await?;

        match &parent.content {
            runtimelib::JupyterMessageContent::CommInfoRequest(_) => {
                // Just tell the frontend we don't have any comms
                let reply = CommInfoReply {
                    status: runtimelib::ReplyStatus::Ok,
                    comms: Default::default(),
                    error: None,
                }
                .as_child_of(parent);
                shell.send(reply).await?;
            }
            runtimelib::JupyterMessageContent::CompleteRequest(req) => {
                let reply = self.complete(req).await?;
                shell.send(reply.as_child_of(parent)).await?;
            }
            runtimelib::JupyterMessageContent::ExecuteRequest(_) => {
                // Respond back with reply immediately
                let reply = ExecuteReply {
                    status: runtimelib::ReplyStatus::Ok,
                    execution_count: self.one_up_execution_count(),
                    user_expressions: Default::default(),
                    payload: Default::default(),
                    error: None,
                }
                .as_child_of(parent);
                shell.send(reply).await?;

                if let Err(err) = self.execute(parent).await {
                    self.send_error("OllamaFailure", &err.to_string(), parent)
                        .await?;
                }
            }
            runtimelib::JupyterMessageContent::HistoryRequest(_) => {
                let reply = HistoryReply {
                    history: Default::default(),
                    status: runtimelib::ReplyStatus::Ok,
                    error: None,
                }
                .as_child_of(parent);
                shell.send(reply).await?;
            }
            runtimelib::JupyterMessageContent::InspectRequest(_) => {
                // Would be really cool to have the model inspect at the word,
                // kind of like an editor.

                let reply = InspectReply {
                    found: false,
                    data: Media::default(),
                    metadata: Default::default(),
                    status: runtimelib::ReplyStatus::Ok,
                    error: None,
                }
                .as_child_of(parent);

                shell.send(reply).await?;
            }
            runtimelib::JupyterMessageContent::IsCompleteRequest(_) => {
                // true, unconditionally
                let reply = IsCompleteReply {
                    status: IsCompleteReplyStatus::Complete,
                    indent: "".to_string(),
                }
                .as_child_of(parent);

                shell.send(reply).await?;
            }
            runtimelib::JupyterMessageContent::KernelInfoRequest(_) => {
                let reply = Self::kernel_info().as_child_of(parent);

                shell.send(reply).await?;
            }
            // Not implemented for shell includes DebugRequest
            // Not implemented for control (and sometimes shell...) includes InterruptRequest, ShutdownRequest
            _ => {}
        };

        self.iopub.send(Status::idle().as_child_of(parent)).await?;

        Ok(())
    }

    fn kernel_info() -> KernelInfoReply {
        KernelInfoReply {
            status: ReplyStatus::Ok,
            protocol_version: "5.3".to_string(),
            implementation: "Ollama Kernel".to_string(),
            implementation_version: "0.1".to_string(),
            language_info: LanguageInfo {
                name: "markdown".to_string(),
                version: "0.1".to_string(),
                mimetype: "text/markdown".to_string(),
                file_extension: ".md".to_string(),
                pygments_lexer: "markdown".to_string(),
                codemirror_mode: CodeMirrorMode::Simple("markdown".to_string()),
                nbconvert_exporter: "script".to_string(),
            },
            banner: "Ollama Kernel".to_string(),
            help_links: vec![
                HelpLink {
                    text: "Ollama".to_string(),
                    url: "https://ollama.ai".to_string(),
                },
                HelpLink {
                    text: "Local Ollama Server".to_string(),
                    url: OLLAMA_ENDPOINT.to_string(),
                },
            ],
            debugger: false,
            error: None,
        }
    }

    fn one_up_execution_count(&mut self) -> ExecutionCount {
        self.execution_count.0 += 1;
        self.execution_count
    }
}

pub async fn start_kernel(connection_filepath: &str) -> anyhow::Result<()> {
    let conn_file = std::fs::read_to_string(connection_filepath)
        .with_context(|| format!("Couldn't read connection file: {:?}", connection_filepath))?;
    let spec: ConnectionInfo = serde_json::from_str(&conn_file).with_context(|| {
        format!(
            "Connection file is not a valid JSON: {:?}",
            connection_filepath
        )
    })?;

    println!("Starting Ollama Kernel 🦙🌽");
    OllamaKernel::start("llama3.2:1b".to_string(), &spec).await?;

    anyhow::Ok(())
}

#[tokio::main]
async fn main() {
    // Parse the connection file path from the command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <connection_file>", args[0]);
        std::process::exit(1);
    }
    let connection_filepath = &args[1];

    let started = start_kernel(connection_filepath).await;

    if let Err(e) = started {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
