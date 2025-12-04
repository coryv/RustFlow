use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub active_team_id: Option<Uuid>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Config::get_path()?;
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Config::get_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn get_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "rustflow", "cli")
            .ok_or_else(|| anyhow!("Could not determine config directory"))?;
        Ok(proj_dirs.config_dir().join("config.toml"))
    }
}
