use tokio::io::{self};
// use tokio::net::TcpStream;
// use reqwest::Client;
use clap::Parser;
use clap::Subcommand;

use tabled::{
    settings::{object::Rows, themes::Colorization, Color, Style},
    Table, Tabled,
};

// TODO: Rely on our server for the source of truth rather than the runtimelib
use runtimelib::jupyter_runtime::get_jupyter_runtime_instances;

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
        id: u32,
    },
    /* TODO: Start a REPL session
    // Run {
    //     /// The REPL to start (e.g., python3, node)
    //     repl: String,
    // },
     */
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { name } => {
            create_instance(name).await?;
        }
        Commands::Ps => {
            list_instances().await?;
        }
        Commands::Kill { id } => {
            kill_instance(id).await?;
        } // Commands::Run { repl } => {
          //     start_repl(repl).await?;
          // }
    }

    Ok(())
}

async fn create_instance(name: String) -> io::Result<()> {
    println!("No runtime for: {}", name);
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No runtime for: {}", name),
    ))
}

#[derive(Tabled)]
struct RuntimeDisplay {
    #[tabled(rename = "Kernel Name")]
    kernel_name: String,
    #[tabled(rename = "IP")]
    ip: String,
    #[tabled(rename = "Transport")]
    transport: String,
    #[tabled(rename = "Connection File")]
    connection_file: String,
}

async fn list_instances() -> io::Result<()> {
    // equivalent to `docker ps`, presumably we'd want to get `docker ps --all`
    //
    // runt ps
    // runt ps --all

    // kernels vs kernelspecs
    // GET  localhost:63409/v1/instances

    let runtimes = get_jupyter_runtime_instances().await;

    let displays: Vec<RuntimeDisplay> = runtimes
        .into_iter()
        .map(|runtime| RuntimeDisplay {
            kernel_name: runtime.connection_info.kernel_name.unwrap_or("".to_string()),
            ip: runtime.connection_info.ip,
            transport: runtime.connection_info.transport,
            connection_file: runtime.connection_file,
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

async fn kill_instance(id: u32) -> io::Result<()> {
    println!("No runtime running with ID: {}", id);
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Runtime with ID {} not found", id),
    ))
}
