use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vaxis")]
#[command(about = "Vaxis CLI — your AI-powered developer tool")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output raw JSON (for scripting and AI agents)
    #[arg(long, global = true)]
    pub json: bool,
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

    /// Manage your applications
    Apps {
        #[command(subcommand)]
        action: AppsAction,
    },

    /// Manage diagrams within an application
    Diagrams {
        #[command(subcommand)]
        action: DiagramsAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set the Vaxis server URL (e.g. http://localhost:3000)
    SetUrl { url: String },

    /// Show current configuration
    Show,
}

#[derive(Subcommand)]
pub enum AppsAction {
    /// List all your applications
    List,

    /// Create a new application
    Create {
        /// Application name
        name: String,

        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Update an application's name or description (interactive if no ID given)
    Update {
        /// Application ID (omit to pick from list)
        id: Option<String>,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete an application (interactive if no ID given)
    Delete {
        /// Application ID (omit to pick from list)
        id: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Get or create the public shareable link for an application
    Share {
        /// Application ID
        id: String,
    },
}

#[derive(Subcommand)]
pub enum DiagramsAction {
    /// List all diagrams in an application
    List {
        /// Application ID
        app_id: String,
    },

    /// Create a new diagram in an application
    Create {
        /// Application ID
        app_id: String,

        /// Diagram name
        name: String,
    },

    /// Generate or update a diagram — use --prompt (server AI) or --mermaid (Claude AI)
    Generate {
        /// Diagram ID
        id: String,

        /// Let the server AI generate the diagram from this prompt
        #[arg(short, long, conflicts_with = "mermaid")]
        prompt: Option<String>,

        /// Provide Mermaid directly — server skips AI, still processes drill annotations
        #[arg(short, long, conflicts_with = "prompt")]
        mermaid: Option<String>,
    },

    /// Show a diagram's content and structure
    Show {
        /// Diagram ID
        id: String,
    },

    /// Show the full diagram tree for an application
    Tree {
        /// Any diagram ID in the application (navigates to root automatically)
        id: String,
    },

    /// Remove the last AI generation turn (safe undo before retry)
    Undo {
        /// Diagram ID
        id: String,
    },

    /// Rename a diagram
    Rename {
        /// Diagram ID
        id: String,

        /// New name
        name: String,
    },

    /// Delete a diagram and all its children (interactive if no ID given)
    Delete {
        /// Diagram ID (omit to pick from list)
        id: Option<String>,

        /// Application ID (used for interactive picker)
        #[arg(long)]
        app_id: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Return Mermaid format reference — diagram types, syntax rules, limits
    Format,

    /// Apply a targeted diff to a diagram without rewriting the full Mermaid
    Patch {
        /// Diagram ID
        id: String,

        /// JSON diff: {"add_nodes":[...],"add_edges":[...],"remove_nodes":[...],"remove_edges":[...],"update_labels":[...]}
        #[arg(long)]
        diff: String,
    },

    /// Save raw Mermaid to a diagram directly, bypassing AI
    Import {
        /// Diagram ID
        id: String,

        /// Raw Mermaid source to save
        #[arg(long)]
        mermaid: String,
    },
}
