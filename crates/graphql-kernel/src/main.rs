use std::collections::HashMap;

use clap::Parser;
use jupyter_protocol::{
    ConnectionInfo, ErrorOutput, ExecuteReply, ExecuteRequest, ExecutionCount, IsCompleteReply,
    IsCompleteReplyStatus, JupyterMessage, JupyterMessageContent, KernelInfoReply, Media,
    MediaType, ReplyStatus, Status, StreamContent,
};
use reqwest::Client;
use runtimelib::{DisplayData, KernelIoPubConnection, KernelShellConnection};
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

struct GraphQLKernel {
    server_url: String,
    headers: HashMap<String, String>,
    client: Client,
    iopub: KernelIoPubConnection,
    execution_count: ExecutionCount,
}

impl GraphQLKernel {
    fn new(iopub: KernelIoPubConnection) -> Self {
        Self {
            server_url: "https://countries.trevorblades.com/graphql".to_string(),
            headers: HashMap::new(),
            client: Client::new(),
            iopub,
            execution_count: ExecutionCount::new(0),
        }
    }

    fn kernel_info_reply(&mut self) -> KernelInfoReply {
        KernelInfoReply {
            status: ReplyStatus::Ok,
            protocol_version: "5.3".to_string(),
            implementation: "graphql-kernel".to_string(),
            implementation_version: env!("CARGO_PKG_VERSION").to_string(),
            language_info: jupyter_protocol::LanguageInfo {
                name: "graphql".to_string(),
                version: "June 2018".to_string(),
                mimetype: "application/graphql".to_string(),
                file_extension: ".graphql".to_string(),
                pygments_lexer: "graphql".to_string(),
                codemirror_mode: jupyter_protocol::CodeMirrorMode::Simple("graphql".to_string()),
                nbconvert_exporter: "".to_string(),
            },
            banner: "GraphQL Kernel".to_string(),
            help_links: vec![],
            debugger: false,
            error: None,
        }
    }

    async fn execute(
        &mut self,
        request: &ExecuteRequest,
        parent_message: &JupyterMessage,
    ) -> anyhow::Result<()> {
        if request.code.starts_with("%") {
            self.handle_magic(&request.code, parent_message).await?;
        } else {
            self.execute_graphql(&request.code, parent_message).await?;
        }

        Ok(())
    }

    async fn handle_magic(
        &mut self,
        code: &str,
        _parent_message: &JupyterMessage,
    ) -> anyhow::Result<()> {
        let command = code.trim_start_matches('%');
        let parts: Vec<&str> = command.split_whitespace().collect();
        match parts.get(0) {
            Some(&"server") => self.handle_server_command(&parts[1..]).await?,
            Some(&"headers") => self.handle_headers_command(&parts[1..]).await?,
            // Implement other magic commands
            //// Other potential magic commands:
            // %schema - Fetch and display the GraphQL schema
            // %variables - Set, clear, or show variables for queries
            // %history - Show recent queries
            // %persist - Save, load, or list persistent queries
            // %timing - Toggle display of request execution time
            //
            _ => return Err(anyhow::anyhow!("Unknown magic command")),
        }
        Ok(())
    }

    async fn execute_graphql(
        &mut self,
        code: &str,
        parent_message: &JupyterMessage,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .post(&self.server_url)
            .headers((&self.headers).try_into()?)
            .json(&json!({ "query": code }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            let json: serde_json::Value = serde_json::from_str(&body)?;
            let formatted = serde_json::to_string_pretty(&json)?;

            let media = Media::new(vec![
                MediaType::Json(json.as_object().unwrap().clone()),
                MediaType::Plain(formatted),
            ]);

            self.iopub
                .send(DisplayData::new(media).as_child_of(parent_message))
                .await?;
        } else {
            self.iopub
                .send(
                    ErrorOutput {
                        ename: "GraphQLError".to_string(),
                        evalue: format!("HTTP {}: {}", status, body),
                        traceback: vec![],
                    }
                    .as_child_of(parent_message),
                )
                .await?;
        }

        Ok(())
    }

    async fn handle_server_command(&mut self, args: &[&str]) -> anyhow::Result<()> {
        // %server set <url>
        match args.get(0) {
            Some(&"set") => {
                if let Some(url) = args.get(1) {
                    self.server_url = url.to_string();
                    // Send confirmation message
                } else {
                    // Send error message
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown server command")),
        }
        Ok(())
    }

    async fn handle_headers_command(&mut self, args: &[&str]) -> anyhow::Result<()> {
        // %headers set <key> <value>
        // %headers clear
        // %headers show
        Ok(())
    }
    async fn handle_message(
        &mut self,
        msg: &JupyterMessage,
        shell: &mut KernelShellConnection,
    ) -> anyhow::Result<()> {
        self.iopub.send(Status::busy().as_child_of(msg)).await?;

        match &msg.content {
            JupyterMessageContent::ExecuteRequest(req) => {
                self.execution_count.increment();
                let reply = ExecuteReply {
                    status: ReplyStatus::Ok,
                    execution_count: self.execution_count.clone(),
                    ..Default::default()
                };
                shell.send(reply.as_child_of(msg)).await?;

                self.execute(req, msg).await?;
            }
            JupyterMessageContent::KernelInfoRequest(_) => {
                let reply = self.kernel_info_reply();
                shell.send(reply.as_child_of(msg)).await?;
            }
            JupyterMessageContent::IsCompleteRequest(_req) => {
                let reply = IsCompleteReply {
                    status: IsCompleteReplyStatus::Complete,
                    indent: String::new(),
                };
                shell.send(reply.as_child_of(msg)).await?;
            }
            JupyterMessageContent::HistoryRequest(_) => {
                let reply = jupyter_protocol::HistoryReply {
                    // sticking this in as an example for now
                    history: vec![jupyter_protocol::HistoryEntry::Input(
                        0,
                        0,
                        r#"query Query {
  country(code: "BR") {
    name
    native
    capital
    emoji
    currency
    languages {
      code
      name
    }
  }
}"#
                        .to_string(),
                    )],
                    status: jupyter_protocol::ReplyStatus::Ok,
                    error: None,
                };
                shell.send(reply.as_child_of(msg)).await?;
            }
            JupyterMessageContent::CompleteRequest(req) => {
                shell
                    .send(
                        jupyter_protocol::CompleteReply {
                            matches: vec![],
                            cursor_start: req.cursor_pos,
                            cursor_end: req.cursor_pos,
                            metadata: serde_json::Map::new(),
                            status: jupyter_protocol::ReplyStatus::Ok,
                            error: None,
                        }
                        .as_child_of(msg),
                    )
                    .await?;
            }
            _ => {}
        }

        self.iopub.send(Status::idle().as_child_of(msg)).await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the connection file
    #[arg(short, long)]
    connection_file: Option<String>,

    /// Install the kernel
    #[arg(long)]
    install: bool,
}

async fn install_kernel() -> anyhow::Result<()> {
    println!("Installing GraphQL Kernel...");

    let user_data_dir = runtimelib::user_data_dir()?;
    let kernel_dir = user_data_dir.join("kernels").join("graphql");

    fs::create_dir_all(&kernel_dir).await?;

    let kernel_json_path = kernel_dir.join("kernel.json");

    let json_data = json!({
        "argv": [std::env::current_exe()?.to_string_lossy().to_string(), "--connection-file", "{connection_file}"],
        "display_name": "GraphQL",
        "language": "graphql",
    });

    fs::write(kernel_json_path, serde_json::to_string_pretty(&json_data)?).await?;

    println!("GraphQL Kernel installed successfully!");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.install {
        install_kernel().await?;

        return Ok(());
    }

    let connection_file = if let Some(connection_file) = args.connection_file {
        connection_file
    } else {
        anyhow::bail!("Either --install or --connection-file must be provided");
    };

    eprintln!("Using connection file: {connection_file}");

    let connection_info: ConnectionInfo =
        serde_json::from_str(&fs::read_to_string(&connection_file).await?)?;

    let session_id = Uuid::new_v4().to_string();

    let mut heartbeat = runtimelib::create_kernel_heartbeat_connection(&connection_info).await?;
    let mut shell =
        runtimelib::create_kernel_shell_connection(&connection_info, &session_id).await?;
    let iopub = runtimelib::create_kernel_iopub_connection(&connection_info, &session_id).await?;

    let mut kernel = GraphQLKernel::new(iopub);

    tokio::spawn(async move {
        loop {
            if let Err(e) = heartbeat.single_heartbeat().await {
                eprintln!("Heartbeat error: {}", e);
                break;
            }
        }
    });

    loop {
        match shell.read().await {
            Ok(msg) => {
                if let Err(e) = kernel.handle_message(&msg, &mut shell).await {
                    eprintln!("Error handling message: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }

    Ok(())
}
