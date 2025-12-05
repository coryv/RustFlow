use clap::{Parser, Subcommand};
use uuid::Uuid;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Remote server URL (e.g., http://localhost:3000)
    #[arg(long, global = true)]
    pub server: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize the database
    InitDb {
        /// Database URL (default: sqlite:rustflow.db)
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    
    // --- Authentication ---
    /// Register a new user
    Register {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Login as a user
    Login {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Logout (clear local session)
    Logout,
    /// Show current logged in user and active team
    Whoami,

    // --- Team Management ---
    /// Manage teams
    Team {
        #[command(subcommand)]
        cmd: TeamCommands,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },

    // --- Workflow & Resources ---
    /// Run a workflow from a file
    Run {
        /// Path to workflow file (YAML/JSON)
        #[arg(short, long)]
        file: String,
        /// Team ID (optional, defaults to active team)
        #[arg(short, long)]
        team_id: Option<Uuid>,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
        /// Enable debug mode (implies limit=1 if not specified)
        #[arg(long)]
        debug: bool,
        /// Limit number of records per step
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Save a workflow to the database
    SaveWorkflow {
        /// Path to the workflow YAML file
        #[arg(short, long)]
        file: PathBuf,
        /// Team ID (optional, defaults to active team)
        #[arg(short, long)]
        team_id: Option<Uuid>,
        /// Workflow name
        #[arg(short, long)]
        name: String,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// List workflows for a team
    ListWorkflows {
        /// Team ID (optional, defaults to active team)
        #[arg(short, long)]
        team_id: Option<Uuid>,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Create a new credential
    CreateCredential {
        /// Credential name
        #[arg(short, long)]
        name: String,
        /// Credential type (e.g., openai_api)
        #[arg(short = 't', long)]
        credential_type: String,
        /// Secret data
        #[arg(short, long)]
        data: String,
        /// Team ID (optional, defaults to active team)
        #[arg(short, long)]
        team_id: Option<Uuid>,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// List credentials for a team
    ListCredentials {
        /// Team ID (optional, defaults to active team)
        #[arg(short, long)]
        team_id: Option<Uuid>,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    #[command(about = "List available nodes")]
    ListNodes,
    #[command(about = "Start interactive workflow builder")]
    Build,
    
    #[command(about = "Manage an existing workflow")]
    Manage {
        /// Path to workflow file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TeamCommands {
    /// Create a new team
    Create {
        #[arg(short, long)]
        name: String,
    },
    /// List teams you are a member of
    List,
    /// Add a member to a team (Admin only)
    AddMember {
        #[arg(short, long)]
        team_id: Uuid,
        #[arg(short, long)]
        username: String,
        #[arg(short, long, default_value = "member")]
        role: String,
    },
    /// Switch active team context
    Switch {
        #[arg(short, long)]
        team_id: Uuid,
    },
}
