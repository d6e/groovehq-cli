mod auth;

pub use auth::resolve_token;

use crate::error::{GrooveError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_token: Option<String>,
    pub api_endpoint: Option<String>,

    #[serde(default)]
    pub defaults: DefaultSettings,

    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    pub format: Option<String>,
    pub limit: Option<u32>,
    pub folder: Option<String>,
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("com", "groovehq", "cli")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = match Self::path() {
            Some(p) => p,
            None => return Ok(Config::default()),
        };

        if !path.exists() {
            return Ok(Config::default());
        }

        let contents = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().ok_or_else(|| {
            GrooveError::Config("Could not determine config directory".into())
        })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| GrooveError::Config(e.to_string()))?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn set_token(&mut self, token: String) -> Result<()> {
        self.api_token = Some(token);
        self.save()
    }
}
