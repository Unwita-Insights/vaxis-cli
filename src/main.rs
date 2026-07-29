mod cli;
mod config;
mod commands;
mod mermaid_lint;
mod parity_eval;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if msg.contains("BrokenPipe")
            || msg.contains("os error 32")
            || msg.contains("os error 232")
        {
            std::process::exit(0);
        }
        eprintln!("{info}");
    }));

    let cli = Cli::parse();
    let json = cli.json;

    match cli.command {
        Commands::Login                => commands::login::run().await,
        Commands::Install { skills, agent, project, global, yes, force } =>
            commands::skills::install(skills, agent, project, global, yes, force, json),
        Commands::Skills { action }     => commands::skills::run(action, json),
        Commands::Me                   => commands::me::run(json),
        Commands::Logout               => commands::logout::run(),
        Commands::Config { action }    => commands::config::run(action, json),
        Commands::Apps   { action }    => commands::apps::run(action, json).await,
        Commands::Diagrams { action }  => commands::diagrams::run(action, json).await,
        Commands::Upgrade              => commands::upgrade::run(json).await,
        Commands::Uninstall { force }  => commands::uninstall::run(force, json),
    }
}
