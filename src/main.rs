mod cli;
mod config;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login             => commands::login::run().await,
        Commands::Me                => commands::me::run(),
        Commands::Logout            => commands::logout::run(),
        Commands::Config { action } => commands::config::run(action),
    }
}
