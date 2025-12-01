use clap::{Parser, Subcommand};
use rust_flow::schema::{WorkflowDefinition, WorkflowLoader};
use rust_flow::storage::{Storage, SqliteStorage, WorkflowEntity};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use uuid::Uuid;
use chrono::Utc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a workflow from a file
    Run {
        /// Path to workflow file (YAML/JSON)
        #[arg(short, long)]
        file: String,
        /// Account ID (optional, for loading credentials)
        #[arg(short, long)]
        account_id: Option<Uuid>,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Initialize the database
    InitDb {
        /// Database URL (default: sqlite:rustflow.db)
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Create a new account
    CreateAccount {
        /// Account name
        #[arg(short, long)]
        name: String,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// Save a workflow to the database
    SaveWorkflow {
        /// Path to the workflow YAML file
        #[arg(short, long)]
        file: PathBuf,
        /// Account ID to associate with
        #[arg(short, long)]
        account_id: Uuid,
        /// Workflow name
        #[arg(short, long)]
        name: String,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// List workflows for an account
    ListWorkflows {
        /// Account ID
        #[arg(short, long)]
        account_id: Uuid,
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
        /// Account ID
        #[arg(short, long)]
        account_id: Uuid,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
    /// List credentials for an account
    ListCredentials {
        /// Account ID
        #[arg(short, long)]
        account_id: Uuid,
        /// Database URL
        #[arg(long, default_value = "sqlite:rustflow.db")]
        db_url: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Run { file, account_id, db_url } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {}", file))?;
            
            println!("Loading workflow from: {:?}", file);
            let loader = WorkflowLoader::new();
            let workflow_def = loader.load(&content)?;
            
            println!("Workflow parsed successfully. Building graph...");

            // Load credentials if account_id is provided
            let mut secrets = std::collections::HashMap::new();
            if let Some(acc_id) = account_id {
                let storage = SqliteStorage::new(&db_url).await?;
                let creds = storage.list_credentials(acc_id).await?;
                for cred in creds {
                    match rust_flow::storage::encryption::decrypt(&cred.data) {
                        Ok(decrypted) => {
                            secrets.insert(cred.id.to_string(), decrypted);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to decrypt credential {}: {}", cred.id, e);
                        }
                    }
                }
                println!("Loaded {} credentials.", secrets.len());
            }

            let mut executor = workflow_def.to_executor(&secrets)?;
            
            println!("Starting execution...");
            executor.run().await?;
            println!("Execution finished.");
        }
        Commands::InitDb { db_url } => {
            // Ensure file exists for sqlite
            if db_url.starts_with("sqlite:") {
                let path = db_url.trim_start_matches("sqlite:");
                if !std::path::Path::new(path).exists() {
                    fs::File::create(path)?;
                }
            }
            let storage = SqliteStorage::new(&db_url).await?;
            storage.init().await?;
            println!("Database initialized at {}", db_url);
        }
        Commands::CreateAccount { name, db_url } => {
            let storage = SqliteStorage::new(&db_url).await?;
            let account = storage.create_account(&name).await?;
            println!("Account created: {} (ID: {})", account.name, account.id);
        }
        Commands::SaveWorkflow { file, account_id, name, db_url } => {
            let storage = SqliteStorage::new(&db_url).await?;
            let content = fs::read_to_string(&file)?;
            let definition: serde_json::Value = serde_yaml::from_str(&content)?; // Store as JSON Value
            
            let workflow = WorkflowEntity {
                id: Uuid::new_v4(),
                account_id,
                name,
                definition,
                created_at: Utc::now(),
            };
            
            storage.save_workflow(&workflow).await?;
            println!("Workflow saved: {} (ID: {})", workflow.name, workflow.id);
        }
        Commands::ListWorkflows { account_id, db_url } => {
            let storage = SqliteStorage::new(&db_url).await?;
            let workflows = storage.list_workflows(account_id).await?;
            println!("Workflows for Account {}:", account_id);
            for wf in workflows {
                println!("- {} (ID: {})", wf.name, wf.id);
            }
        }
        Commands::CreateCredential { name, credential_type, data, account_id, db_url } => {
            let storage = SqliteStorage::new(&db_url).await?;
            let cred = storage.create_credential(&name, &credential_type, &data, account_id).await?;
            println!("Credential created: {} (ID: {})", cred.name, cred.id);
        }
        Commands::ListCredentials { account_id, db_url } => {
            let storage = SqliteStorage::new(&db_url).await?;
            let creds = storage.list_credentials(account_id).await?;
            println!("Credentials for Account {}:", account_id);
            for cred in creds {
                println!("- {} (Type: {}, ID: {})", cred.name, cred.credential_type, cred.id);
            }
        }
    }

    Ok(())
}
