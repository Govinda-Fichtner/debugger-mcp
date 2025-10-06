use clap::{Parser, Subcommand};
use debugger_mcp::Result;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "debugger_mcp")]
#[command(about = "DAP-based MCP debugging server", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server listening on STDIO
    Serve {
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Set log level (trace, debug, info, warn, error)
        #[arg(long, default_value = "info")]
        log_level: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { verbose, log_level } => {
            // Initialize tracing
            let level = if verbose { "debug" } else { &log_level };
            let filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(level));

            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .init();

            // Run the server
            debugger_mcp::serve().await?;
        }
    }

    Ok(())
}
