use anyhow::Result;
use clap::Parser;

#[path = "../cli/mod.rs"]
mod cli;

use cli::commands::Args;
use cli::handlers::handle_command;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    if let Some(cmd) = args.command {
        handle_command(Args { command: Some(cmd), ..args }).await
    } else {
        cli::interactive::run().await
    }
}
