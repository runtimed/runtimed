use tokio::io::{self};
// use tokio::net::TcpStream;
use clap::Parser;
use clap::Subcommand;
use std::collections::HashMap;

use runtimelib::jupyter::client::JupyterRuntime;

use anyhow::Error;

use tabled::{
    settings::{object::Rows, themes::Colorization, Color, Style},
    Table, Tabled,
};

/** Runtime 🔄  */
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Commands to interact with runtimes
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Start {
        name: String,
    },
    /// List running runtimes
    Ps,
    /// Kill a specific runtime
    Kill {
        /// ID of the runtime to kill
        id: String,
    },
    RunCode {
        id: String,
        code: String,
    },
    Execution {
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { name } => {
            create_instance(name).await?;
        }
        Commands::Ps => {
            list_instances().await?;
        }
        Commands::RunCode { id, code } => {
            run_code(id, code).await?;
        }
        Commands::Execution { id } => {
            execution(id).await?;
        }
        Commands::Kill { id } => {
            kill_instance(id).await?;
        } // Commands::Run { repl } => {
          //     start_repl(repl).await?;
          // }
    }

    Ok(())
}

async fn create_instance(name: String) -> Result<(), Error> {
    Err(Error::msg(format!("No runtime for: {}", name)))
}

#[derive(Tabled)]
struct RuntimeDisplay {
    #[tabled(rename = "Kernel Name")]
    kernel_name: String,
    #[tabled(rename = "Language")]
    language: String,
    #[tabled(rename = "ID")]
    id: uuid::Uuid,
    #[tabled(rename = "IP")]
    ip: String,
    #[tabled(rename = "Transport")]
    transport: String,
    #[tabled(rename = "Connection File")]
    connection_file: String,
    #[tabled(rename = "State")]
    state: String,
}

async fn list_instances() -> Result<(), Error> {
    let runtimes = reqwest::get("http://127.0.0.1:12397/v0/runtime_instances")
        .await?
        .json::<Vec<JupyterRuntime>>()
        .await?;

    let displays: Vec<RuntimeDisplay> = runtimes
        .into_iter()
        .map(|runtime| RuntimeDisplay {
            kernel_name: runtime.kernel_name.chars().take(15).collect(),
            id: runtime.id,
            ip: runtime.ip,
            transport: runtime.transport,
            connection_file: runtime.connection_file,
            state: runtime.state,
            language: runtime.kernel_info["language_info"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
        .collect();

    if !displays.is_empty() {
        let table = Table::new(displays)
            .with(Style::markdown())
            .with(Colorization::exact([Color::BOLD], Rows::first()))
            .to_string();
        println!("{}", table);
    } else {
        println!("No Jupyter runtimes running.");
    }

    Ok(())
}

async fn run_code(id: String, code: String) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:12397/v0/runtime_instances/{}/run_code",
            id,
        ))
        .json(&HashMap::from([("code", code)]))
        .send()
        .await?
        .text()
        .await?;

    println!("{}", response);

    Ok(())
}

async fn execution(id: String) -> Result<(), Error> {
    let response = reqwest::get(format!("http://127.0.0.1:12397/v0/executions/{}", id))
        .await?
        .text()
        .await?;

    println!("{}", response);

    Ok(())
}

async fn kill_instance(id: String) -> io::Result<()> {
    println!("No runtime running with ID: {}", id);
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Runtime with ID {} not found", id),
    ))
}
