use crate::config::Config;
use crate::error::{GrooveError, Result};

pub fn resolve_token(cli_token: Option<&str>, config: &Config) -> Result<String> {
    // 1. CLI flag (--token)
    if let Some(token) = cli_token {
        return Ok(token.to_string());
    }

    // 2. Environment variable
    if let Ok(token) = std::env::var("GROOVEHQ_API_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // 3. Config file
    if let Some(token) = &config.api_token {
        return Ok(token.clone());
    }

    Err(GrooveError::TokenNotFound)
}
