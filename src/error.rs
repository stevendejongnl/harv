use thiserror::Error;

#[derive(Error, Debug)]
pub enum HarjiraError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Harvest API error: {0}")]
    Harvest(String),

    #[error("Jira API error: {0}")]
    Jira(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("No Jira tickets found in commits")]
    NoTicketsFound,

    #[error("User cancelled operation")]
    UserCancelled,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("Invalid time entry: {0}")]
    InvalidEntry(String),
}

pub type Result<T> = std::result::Result<T, HarjiraError>;
