use tokio::io::{self};
// use tokio::net::TcpStream;
// use reqwest::Client;
use clap::Parser;
use clap::Subcommand;

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
        }
        // Commands::Run { repl } => {
        //     start_repl(repl).await?;
        // }
    }    

    Ok(())
}

async fn create_instance(name: String) -> io::Result<()> {
    println!("No runtime for: {}", name);
    Err(io::Error::new(io::ErrorKind::NotFound, format!("No runtime for: {}", name))) 
}



async fn list_instances() -> io::Result<()> {
    // equivalent to `docker ps`, presumably we'd want to get `docker ps --all`
    // 
    // runt ps
    // runt ps --all


    // kernels vs kernelspecs
    // GET  localhost:63409/v1/instances


    println!("No runtimes running");
    Ok(())
}

async fn kill_instance(id: u32) -> io::Result<()> {
    println!("No runtime running with ID: {}", id);
    Err(io::Error::new(io::ErrorKind::NotFound, format!("Runtime with ID {} not found", id)))
}

