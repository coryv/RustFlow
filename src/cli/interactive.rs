use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use crate::cli::commands::{Commands, TeamCommands};
use crate::cli::handlers::handle_command;
use crate::cli::builder::BuilderState;
use crate::cli::config::Config;

pub async fn run() -> Result<()> {
    println!("Welcome to RustFlow CLI!");
    
    loop {
        // Refresh config to get current context
        let config = Config::load().unwrap_or_default();
        let user_status = if let Some(u) = &config.username {
            format!("User: {}", u)
        } else {
            "Not Logged In".to_string()
        };
        
        let team_status = if let Some(t) = &config.active_team_id {
            format!("Team: {}", t)
        } else {
            "No Active Team".to_string()
        };

        println!("\n--- RustFlow Dashboard ---");
        println!("{} | {}", user_status, team_status);
        println!("--------------------------");

        let choices = vec![
            "Workflows",
            "Team",
            "Credentials",
            "Authentication",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Action")
            .default(0)
            .items(&choices)
            .interact()?;

        match selection {
            0 => workflow_menu().await?,
            1 => team_menu().await?,
            2 => credential_menu().await?,
            3 => auth_menu().await?,
            4 => break,
            _ => unreachable!(),
        }
    }
    
    println!("Goodbye!");
    Ok(())
}

async fn workflow_menu() -> Result<()> {
    loop {
        let choices = vec![
            "List Workflows",
            "Build Workflow",
            "Run Workflow (File)",
            "Save Workflow (File)",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Workflows")
            .default(0)
            .items(&choices)
            .interact()?;

        match selection {
            0 => {
                // List Workflows
                // We need to construct Args or call handler directly.
                // Since handler takes Args, let's construct a dummy Args wrapper or just call the logic.
                // For now, let's reuse handle_command by constructing the enum.
                // Note: We need db_url. Let's assume default or prompt?
                // For MVP, let's use default "sqlite:rustflow.db" or load from config if we had it there.
                let db_url = "sqlite:rustflow.db".to_string();
                
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::ListWorkflows { team_id: None, db_url }),
                    server: None,
                }).await?;
            },
            1 => {
                // Build
                let mut builder = BuilderState::new();
                builder.run().await?;
            },
            2 => {
                 println!("To run a workflow, please use the command line: rustflow run -f <file>");
                 // TODO: Implement file picker
            },
            3 => {
                 println!("To save a workflow, please use the command line: rustflow save -f <file> -n <name>");
                 // TODO: Implement file picker and name prompt
            },
            4 => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}

async fn team_menu() -> Result<()> {
    loop {
        let choices = vec![
            "List Teams",
            "Switch Team",
            "Create Team",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Team Management")
            .default(0)
            .items(&choices)
            .interact()?;
        
        let db_url = "sqlite:rustflow.db".to_string();

        match selection {
            0 => {
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Team { cmd: TeamCommands::List, db_url }),
                    server: None,
                }).await?;
            },
            1 => {
                // Switch
                // We need to list teams first to let user select, but handle_command just takes ID.
                // For now, let's ask for ID string.
                // Ideally we should fetch teams and show a select.
                use dialoguer::Input;
                let team_id_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Team ID")
                    .interact_text()?;
                
                if let Ok(uuid) = uuid::Uuid::parse_str(&team_id_str) {
                     handle_command(crate::cli::commands::Args {
                        command: Some(Commands::Team { cmd: TeamCommands::Switch { team_id: uuid }, db_url }),
                        server: None,
                    }).await?;
                } else {
                    println!("Invalid UUID");
                }
            },
            2 => {
                use dialoguer::Input;
                let name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("New Team Name")
                    .interact_text()?;
                
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Team { cmd: TeamCommands::Create { name }, db_url }),
                    server: None,
                }).await?;
            },
            3 => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}

async fn credential_menu() -> Result<()> {
    loop {
        let choices = vec![
            "List Credentials",
            "Create Credential",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Credentials")
            .default(0)
            .items(&choices)
            .interact()?;
        
        let db_url = "sqlite:rustflow.db".to_string();

        match selection {
            0 => {
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::ListCredentials { team_id: None, db_url }),
                    server: None,
                }).await?;
            },
            1 => {
                use dialoguer::Input;
                let name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Credential Name")
                    .interact_text()?;
                let ctype: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Type (e.g. openai_api)")
                    .interact_text()?;
                let data: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Secret Data")
                    .interact_text()?;

                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::CreateCredential { 
                        name, 
                        credential_type: ctype, 
                        data, 
                        team_id: None, 
                        db_url 
                    }),
                    server: None,
                }).await?;
            },
            2 => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}

async fn auth_menu() -> Result<()> {
    loop {
        let choices = vec![
            "Who Am I",
            "Login",
            "Register",
            "Logout",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Authentication")
            .default(0)
            .items(&choices)
            .interact()?;
        
        let db_url = "sqlite:rustflow.db".to_string();

        match selection {
            0 => {
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Whoami),
                    server: None,
                }).await?;
            },
            1 => {
                use dialoguer::{Input, Password};
                let username: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Username")
                    .interact_text()?;
                let password = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Password")
                    .interact()?;
                
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Login { username, password, db_url }),
                    server: None,
                }).await?;
            },
            2 => {
                 use dialoguer::{Input, Password};
                let username: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Username")
                    .interact_text()?;
                let password = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Password")
                    .interact()?;
                
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Register { username, password, db_url }),
                    server: None,
                }).await?;
            },
            3 => {
                handle_command(crate::cli::commands::Args {
                    command: Some(Commands::Logout),
                    server: None,
                }).await?;
            },
            4 => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}
