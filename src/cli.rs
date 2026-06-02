use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vaxis")]
#[command(about = "Vaxis CLI — your AI-powered developer tool")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Log in with your Google account
    Login,

    /// Show your stored profile
    Me,

    /// Log out and clear stored credentials
    Logout,

    /// Configure CLI settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set the Vaxis server URL (e.g. http://localhost:3000)
    SetUrl { url: String },

    /// Show current configuration
    Show,
}
