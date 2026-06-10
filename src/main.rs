mod cli;
mod config;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json = cli.json;

    match cli.command {
        Commands::Login                => commands::login::run().await,
        Commands::Me                   => commands::me::run(json),
        Commands::Logout               => commands::logout::run(),
        Commands::Config { action }    => commands::config::run(action),
        Commands::Apps   { action }    => commands::apps::run(action, json).await,
        Commands::Diagrams { action }  => commands::diagrams::run(action, json).await,
    }
}
