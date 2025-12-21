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
        let path = Self::path()
            .ok_or_else(|| GrooveError::Config("Could not determine config directory".into()))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents =
            toml::to_string_pretty(self).map_err(|e| GrooveError::Config(e.to_string()))?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn set_token(&mut self, token: String) -> Result<()> {
        self.api_token = Some(token);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.api_token.is_none());
        assert!(config.api_endpoint.is_none());
        assert!(config.defaults.format.is_none());
        assert!(config.defaults.limit.is_none());
        assert!(config.defaults.folder.is_none());
        assert!(config.aliases.is_empty());
    }

    #[test]
    fn test_config_parse_toml_minimal() {
        let toml_str = r#"
api_token = "test-token"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.api_token, Some("test-token".to_string()));
        assert!(config.api_endpoint.is_none());
    }

    #[test]
    fn test_config_parse_toml_full() {
        let toml_str = r#"
api_token = "test-token"
api_endpoint = "https://custom.api.com/graphql"

[defaults]
format = "json"
limit = 50
folder = "inbox"

[aliases]
ls = "conversation list"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.api_token, Some("test-token".to_string()));
        assert_eq!(
            config.api_endpoint,
            Some("https://custom.api.com/graphql".to_string())
        );
        assert_eq!(config.defaults.format, Some("json".to_string()));
        assert_eq!(config.defaults.limit, Some(50));
        assert_eq!(config.defaults.folder, Some("inbox".to_string()));
        assert_eq!(
            config.aliases.get("ls"),
            Some(&"conversation list".to_string())
        );
    }

    #[test]
    fn test_config_parse_toml_empty() {
        let toml_str = "";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.api_token.is_none());
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let mut config = Config::default();
        config.api_token = Some("secret-token".to_string());
        config.defaults.limit = Some(100);

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(config.api_token, deserialized.api_token);
        assert_eq!(config.defaults.limit, deserialized.defaults.limit);
    }

    #[test]
    fn test_config_path_returns_some() {
        // Config path should return Some on most systems
        let path = Config::path();
        // We just check it doesn't panic and returns a path with config.toml
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("config.toml"));
        }
    }
}
