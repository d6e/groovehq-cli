use thiserror::Error;

#[derive(Error, Debug)]
pub enum GrooveError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("API token not found. Set GROOVEHQ_API_TOKEN or run 'groove config set-token'")]
    TokenNotFound,

    #[error("Conversation #{0} not found")]
    ConversationNotFound(i64),

    #[error("Tag '{0}' not found")]
    TagNotFound(String),

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Canned reply '{0}' not found")]
    CannedReplyNotFound(String),

    #[error("GraphQL error: {0}")]
    GraphQL(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Rate limited{}", match .retry_after {
        Some(secs) => format!(". Retry after {} seconds", secs),
        None => ". Please wait and try again".to_string(),
    })]
    RateLimited { retry_after: Option<u64> },
}

pub type Result<T> = std::result::Result<T, GrooveError>;
