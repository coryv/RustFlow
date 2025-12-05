use anyhow::{Result, Context, anyhow};
use uuid::Uuid;
use chrono::Utc;
use std::fs;
use argon2::PasswordVerifier;
use rust_flow::schema::WorkflowLoader;
use rust_flow::storage::{Storage, SqliteStorage, RemoteStorage, PostgresStorage, WorkflowEntity, Role};
use crate::cli::config::Config;
use crate::cli::commands::{Args, Commands, TeamCommands};
use crate::cli::builder;

async fn get_storage(server: Option<String>, db_url: String) -> Result<Box<dyn Storage>> {
    if let Some(url) = server {
        let storage = RemoteStorage::new(&url);
        storage.init().await?;
        Ok(Box::new(storage))
    } else if db_url.starts_with("postgres://") {
        let storage = PostgresStorage::new(&db_url).await?;
        storage.init().await?;
        Ok(Box::new(storage))
    } else {
        let storage = SqliteStorage::new(&db_url).await?;
        Ok(Box::new(storage))
    }
}

pub async fn handle_command(args: Args) -> Result<()> {
    let command = args.command.ok_or_else(|| anyhow!("No command specified"))?;
    match command {
        Commands::InitDb { db_url } => {
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
        Commands::Register { username, password, db_url } => {
            let storage = get_storage(args.server, db_url).await?;
            let user = storage.create_user(&username, &password).await?;
            println!("User registered: {} (ID: {})", user.username, user.id);
            
            // Auto login
            let mut config = Config::load()?;
            config.user_id = Some(user.id);
            config.username = Some(user.username);
            config.save()?;
            println!("Logged in as {}", username);
        }
        Commands::Login { username, password, db_url } => {
            let storage = get_storage(args.server, db_url).await?;
            let user = storage.get_user_by_username(&username).await?
                .ok_or_else(|| anyhow!("User not found"))?;
            
            // Verify password
            let parsed_hash = argon2::PasswordHash::new(&user.password_hash)
                .map_err(|e| anyhow!("Invalid password hash: {}", e))?;
            
            if argon2::Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                let mut config = Config::load()?;
                config.user_id = Some(user.id);
                config.username = Some(user.username);
                config.active_team_id = None; 
                config.save()?;
                println!("Logged in as {}", username);
            } else {
                return Err(anyhow!("Invalid password"));
            }
        }
        Commands::Logout => {
            let mut config = Config::load()?;
            config.user_id = None;
            config.username = None;
            config.active_team_id = None;
            config.save()?;
            println!("Logged out.");
        }
        Commands::Whoami => {
            let config = Config::load()?;
            if let Some(username) = config.username {
                println!("User: {} (ID: {:?})", username, config.user_id.unwrap());
                if let Some(team_id) = config.active_team_id {
                    println!("Active Team ID: {}", team_id);
                } else {
                    println!("No active team selected.");
                }
            } else {
                println!("Not logged in.");
            }
        }
        Commands::Team { cmd, db_url } => {
            let storage = get_storage(args.server, db_url).await?;
            let mut config = Config::load()?;
            let user_id = config.user_id.ok_or_else(|| anyhow!("Not logged in"))?;

            match cmd {
                TeamCommands::Create { name } => {
                    let team = storage.create_team(&name).await?;
                    storage.add_team_member(team.id, user_id, Role::Admin).await?;
                    println!("Team created: {} (ID: {})", team.name, team.id);
                    
                    config.active_team_id = Some(team.id);
                    config.save()?;
                    println!("Switched to team: {}", team.name);
                }
                TeamCommands::List => {
                    let teams = storage.list_teams_for_user(user_id).await?;
                    println!("Teams for user {}:", config.username.unwrap());
                    for (team, role) in teams {
                        let active = if Some(team.id) == config.active_team_id { "*" } else { " " };
                        println!("{} {} (ID: {}, Role: {:?})", active, team.name, team.id, role);
                    }
                }
                TeamCommands::AddMember { team_id, username, role } => {
                    let members = storage.get_team_members(team_id).await?;
                    let current_member = members.iter().find(|m| m.user_id == user_id)
                        .ok_or_else(|| anyhow!("You are not a member of this team"))?;
                    
                    if current_member.role != Role::Admin {
                        return Err(anyhow!("Only admins can add members"));
                    }

                    let new_user = storage.get_user_by_username(&username).await?
                        .ok_or_else(|| anyhow!("User {} not found", username))?;
                    
                    let role_enum = Role::from_str(&role).ok_or_else(|| anyhow!("Invalid role. Use 'admin' or 'member'"))?;
                    
                    storage.add_team_member(team_id, new_user.id, role_enum).await?;
                    println!("Added {} to team.", username);
                }
                TeamCommands::Switch { team_id } => {
                    let teams = storage.list_teams_for_user(user_id).await?;
                    if teams.iter().any(|(t, _)| t.id == team_id) {
                        config.active_team_id = Some(team_id);
                        config.save()?;
                        println!("Switched to team ID: {}", team_id);
                    } else {
                        return Err(anyhow!("You are not a member of team {}", team_id));
                    }
                }
            }
        }
        Commands::Run { file, team_id, db_url, debug, limit } => {
            let config = Config::load()?;
            let effective_team_id = team_id.or(config.active_team_id);

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {}", file))?;
            
            println!("Loading workflow from: {:?}", file);
            let loader = WorkflowLoader::new();
            let workflow_def = loader.load(&content)?;
            
            println!("Workflow parsed successfully. Building graph...");

            let mut secrets = std::collections::HashMap::new();
            if let Some(tid) = effective_team_id {
                let storage = get_storage(args.server, db_url).await?;
                let creds = storage.list_credentials(tid).await?;
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
                println!("Loaded {} credentials for Team ID {}.", secrets.len(), tid);
            } else {
                println!("No Team ID provided or active. Running without stored credentials.");
            }

            let mut debug_config = rust_flow::stream_engine::DebugConfig::default();
            if debug {
                debug_config.limit_records = Some(limit.unwrap_or(1));
                println!("Debug mode enabled. Limit: {:?}", debug_config.limit_records);
            } else if let Some(l) = limit {
                debug_config.limit_records = Some(l);
                println!("Record limit set to: {}", l);
            }

            let mut executor = workflow_def.to_executor(&secrets, debug_config)?;
            
            let (tx, mut rx) = tokio::sync::broadcast::channel(100);
            executor.set_event_sender(tx);

            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        rust_flow::schema::ExecutionEvent::NodeStart { node_id } => {
                            println!("Node Started: {}", node_id);
                        }
                        rust_flow::schema::ExecutionEvent::NodeFinish { node_id } => {
                            println!("Node Finished: {}", node_id);
                        }
                        rust_flow::schema::ExecutionEvent::EdgeData { from, to, value } => {
                            println!("Data Flow: {} -> {}", from, to);
                            println!("Payload: {}", serde_json::to_string_pretty(&value).unwrap_or_default());
                        }
                        rust_flow::schema::ExecutionEvent::NodeError { node_id, error } => {
                            eprintln!("Node Error ({}): {}", node_id, error);
                        }
                        rust_flow::schema::ExecutionEvent::WorkflowStart { .. } => {
                            println!("Workflow Started");
                        }
                        rust_flow::schema::ExecutionEvent::WorkflowFinish { .. } => {
                            println!("Workflow Finished");
                        }
                    }
                }
            });
            
            println!("Starting execution...");
            executor.run().await?;
            println!("Execution finished.");
        }
        Commands::SaveWorkflow { file, team_id, name, db_url } => {
            let config = Config::load()?;
            let effective_team_id = team_id.or(config.active_team_id)
                .ok_or_else(|| anyhow!("Team ID required. Use --team-id or switch to a team."))?;

            let storage = get_storage(args.server, db_url).await?;
            let content = fs::read_to_string(&file)?;
            let definition: serde_json::Value = serde_yaml::from_str(&content)?; 
            
            let workflow = WorkflowEntity {
                id: Uuid::new_v4(),
                account_id: effective_team_id,
                name,
                definition,
                created_at: Utc::now(),
            };
            
            storage.save_workflow(&workflow).await?;
            println!("Workflow saved: {} (ID: {})", workflow.name, workflow.id);
        }
        Commands::ListWorkflows { team_id, db_url } => {
            let config = Config::load()?;
            let effective_team_id = team_id.or(config.active_team_id)
                .ok_or_else(|| anyhow!("Team ID required. Use --team-id or switch to a team."))?;

            let storage = get_storage(args.server, db_url).await?;
            let workflows = storage.list_workflows(effective_team_id).await?;
            println!("Workflows for Team {}:", effective_team_id);
            for wf in workflows {
                println!("- {} (ID: {})", wf.name, wf.id);
            }
        }
        Commands::CreateCredential { name, credential_type, data, team_id, db_url } => {
            let config = Config::load()?;
            let effective_team_id = team_id.or(config.active_team_id)
                .ok_or_else(|| anyhow!("Team ID required. Use --team-id or switch to a team."))?;

            let storage = get_storage(args.server, db_url).await?;
            let cred = storage.create_credential(&name, &credential_type, &data, effective_team_id).await?;
            println!("Credential created: {} (ID: {})", cred.name, cred.id);
        }
        Commands::ListCredentials { team_id, db_url } => {
            let config = Config::load()?;
            let effective_team_id = team_id.or(config.active_team_id)
                .ok_or_else(|| anyhow!("Team ID required. Use --team-id or switch to a team."))?;

            let storage = get_storage(args.server, db_url).await?;
            let creds = storage.list_credentials(effective_team_id).await?;
            println!("Credentials for Team {}:", effective_team_id);
            for cred in creds {
                println!("- {} (Type: {}, ID: {})", cred.name, cred.credential_type, cred.id);
            }
        }
        Commands::ListNodes => {
            let registry = rust_flow::node_registry::get_node_registry();
            println!("Available Nodes:");
            for node in registry {
                println!("- {} (ID: {})", node.label, node.id);
            }
        }
        Commands::Build => {
            let mut builder = builder::BuilderState::new();
            builder.run().await?;
        }
        Commands::Manage { file } => {
            match file {
                Some(path) => {
                     let content = fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read file: {:?}", path))?;
                     let loader = WorkflowLoader::new();
                     let def = loader.load(&content)?;
                     println!("Loaded workflow from {:?}", path);
                     
                     let mut builder = builder::BuilderState::from_definition(def);
                     builder.run().await?;
                     
                     // Helper: after managing, we might want to save back?
                     // Builder has "Save Workflow", which asks for simplified filename.
                     // The user might want to save over the original file.
                     // But Builder's save logic is generic. 
                     // For now, assume user manually saves using the menu.
                },
                None => {
                    // TODO: Implement picking from DB/List
                    println!("No file specified. Use --file <path> to manage a local workflow.");
                }
            }
        }
    }
    Ok(())
}
