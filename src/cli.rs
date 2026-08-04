use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vaxis")]
#[command(about = "Vaxis CLI — your AI-powered developer tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output raw JSON (for scripting and AI agents)
    #[arg(long, global = true)]
    pub json: bool,
}

/// Server-AI intent for `diagrams generate --prompt`. Validated here so a typo
/// is rejected before any network call. The value maps 1:1 to the string the
/// backend's `/generate` endpoint expects.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Intent {
    Auto,
    Edit,
    Replace,
    Drill,
    Detail,
    Simplify,
    Ask,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Intent::Auto => "auto",
            Intent::Edit => "edit",
            Intent::Replace => "replace",
            Intent::Drill => "drill",
            Intent::Detail => "detail",
            Intent::Simplify => "simplify",
            Intent::Ask => "ask",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DirectDirectionPolicy {
    Preserve,
    Auto,
}

impl DirectDirectionPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Auto => "auto",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FlowDirection {
    Lr,
    Tb,
}

impl FlowDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lr => "LR",
            Self::Tb => "TB",
        }
    }
}

/// How diagrams are generated. Mirrors the `Intent` pattern above: a `ValueEnum`
/// so clap validates the flag at parse time, with `as_str()` for the string
/// form stored in config.toml.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GenerationMode {
    /// The driving AI (Claude / Codex) writes the Mermaid itself
    Mermaid,
    /// Vaxis's server AI generates the diagram
    Prompt,
    /// Clear the saved generation mode (you will be asked again on next generate)
    Unset,
}

impl GenerationMode {
    pub fn as_str(self) -> Option<&'static str> {
        match self {
            GenerationMode::Mermaid => Some("mermaid"),
            GenerationMode::Prompt => Some("prompt"),
            GenerationMode::Unset => None,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the Vaxis skill for your AI agent
    Install {
        /// Install the Vaxis discovery skill for supported AI agents
        #[arg(long)]
        skills: bool,

        /// Target agent (repeatable; Codex installs use the shared .agents path)
        #[arg(long, value_enum)]
        agent: Vec<SkillAgent>,

        /// Install in the current project
        #[arg(long, conflicts_with = "global")]
        project: bool,

        /// Install for the current user
        #[arg(long, conflicts_with = "project")]
        global: bool,

        /// Accept safe defaults without prompting
        #[arg(long)]
        yes: bool,

        /// Back up and replace a user-modified discovery skill
        #[arg(long)]
        force: bool,
    },

    /// Inspect skills bundled with the Vaxis CLI
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },

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

    /// Upgrade vaxis to the latest version
    Upgrade,

    /// Remove vaxis from your system
    Uninstall {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SkillAgent {
    Agents,
    Claude,
    Codex,
}

impl SkillAgent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Subcommand)]
pub enum SkillsAction {
    /// List skills bundled with this CLI
    List,

    /// Print a bundled skill as exact raw content
    Get {
        /// Bundled skill name
        name: String,
    },

    /// Show the effective source of a bundled skill
    Path {
        /// Bundled skill name
        name: String,
    },

    /// Preview a bundled skill (alias for get)
    Preview {
        /// Bundled skill name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set the Vaxis server URL (e.g. http://localhost:3000)
    SetUrl { url: String },

    /// Set how diagrams are generated: `mermaid` (your own AI writes them) or `prompt` (Vaxis server AI)
    SetMode { mode: GenerationMode },

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

        /// Server-AI intent for the --prompt path (default: auto)
        #[arg(short, long, conflicts_with = "mermaid", value_enum)]
        intent: Option<Intent>,

        /// Target an existing AI chat session (see `diagrams sessions`)
        #[arg(short, long)]
        session: Option<String>,

        /// Direct Mermaid direction policy; preserve is the backward-compatible default
        #[arg(long, value_enum, requires = "mermaid", conflicts_with = "prompt", hide = true)]
        direction_policy: Option<DirectDirectionPolicy>,

        /// Force lr or tb on direct Mermaid
        #[arg(long, value_enum, requires = "mermaid", conflicts_with = "prompt", hide = true)]
        explicit_direction: Option<FlowDirection>,

        /// Mark direct Mermaid as a fresh generation eligible for opt-in auto direction
        #[arg(long, requires = "mermaid", conflicts_with = "prompt", hide = true)]
        fresh_generation: bool,

        /// Canvas width for direction decisions (requires --viewport-height)
        #[arg(long, requires_all = ["mermaid", "viewport_height"], conflicts_with = "prompt", hide = true)]
        viewport_width: Option<u32>,

        /// Canvas height for direction decisions (requires --viewport-width)
        #[arg(long, requires_all = ["mermaid", "viewport_width"], conflicts_with = "prompt", hide = true)]
        viewport_height: Option<u32>,
    },

    /// Ask a question about a diagram — server AI answers in prose, no edit
    Ask {
        /// Diagram ID
        id: String,

        /// The question to ask
        #[arg(short, long)]
        prompt: String,

        /// Target an existing AI chat session (see `diagrams sessions`)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Manage AI chat sessions for a diagram
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },

    /// Get, create, rotate or revoke a diagram's public share link
    Share {
        /// Diagram ID
        id: String,

        /// Mint a new link, invalidating the existing one
        #[arg(long, conflicts_with = "revoke")]
        rotate: bool,

        /// Turn sharing off for this diagram
        #[arg(long, conflicts_with = "rotate")]
        revoke: bool,
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
        /// Diagram ID (required with --json; omit to pick interactively)
        id: Option<String>,

        /// Application ID (used for interactive picker)
        #[arg(long)]
        app_id: Option<String>,

        /// Skip confirmation prompt (required with --json)
        #[arg(short, long)]
        force: bool,
    },

    /// Read a .vaxis/*.ir.json plan file and print a human-readable diagram summary
    Plan {
        /// Path to the IR JSON file (e.g. .vaxis/architecture-overview.ir.json)
        file: PathBuf,
    },

    /// Mermaid authoring contract (JSON) — diagram types, syntax rules, limits
    Format,

    /// Compare the embedded authoring contract with the connected Vaxis server
    #[command(hide = true)]
    RulesCheck,

    /// Evaluate recorded native/direct Mermaid outputs against the parity catalog
    #[command(hide = true)]
    Evaluate {
        /// JSON file containing recorded outputs for one or more eval cases
        #[arg(long)]
        captures: PathBuf,

        /// Write the JSON report to this path instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Save raw Mermaid to a diagram directly, bypassing AI
    Import {
        /// Diagram ID
        id: String,

        /// Raw Mermaid source to save (conflicts with --file)
        #[arg(long, conflicts_with = "file")]
        mermaid: Option<String>,

        /// Path to a .mmd file to import (conflicts with --mermaid)
        #[arg(long, conflicts_with = "mermaid")]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum SessionsAction {
    /// List the AI chat sessions for a diagram
    List {
        /// Diagram ID
        id: String,
    },

    /// Start a new AI chat session on a diagram
    Create {
        /// Diagram ID
        id: String,

        /// Optional session title
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Rename an AI chat session
    Rename {
        /// Diagram ID
        id: String,

        /// Session ID
        session_id: String,

        /// New title
        title: String,
    },
}
