use crate::error::{HarjiraError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub harvest: HarvestConfig,
    pub jira: JiraConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub ticket_filter: TicketFilterConfig,
    #[serde(default)]
    pub ai: AiConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HarvestConfig {
    pub access_token: String,
    pub account_id: String,
    pub user_agent: String,
    pub project_id: Option<u64>,
    pub task_id: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JiraConfig {
    pub access_token: String,
    pub base_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GitConfig {
    #[serde(default)]
    pub repositories: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TicketFilterConfig {
    /// List of ticket prefixes to ignore (e.g., ["CWE", "CVE"])
    #[serde(default)]
    pub denylist: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AiConfig {
    /// Whether AI generation is enabled
    #[serde(default)]
    pub enabled: bool,

    /// AI provider: "openai" or "anthropic"
    #[serde(default = "default_provider")]
    pub provider: String,

    /// API key for the AI provider
    #[serde(default)]
    pub api_key: String,

    /// Model name (optional, uses provider default)
    pub model: Option<String>,

    /// Target hours per day for time entry generation
    #[serde(default = "default_target_hours")]
    pub target_hours: f64,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_target_hours() -> f64 {
    8.0
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_provider(),
            api_key: String::new(),
            model: None,
            target_hours: default_target_hours(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub auto_stop: bool,
    #[serde(default = "default_true")]
    pub auto_select_single: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            auto_stop: false,
            auto_select_single: true,
        }
    }
}

impl Config {
    /// Load configuration from file or create template
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Err(HarjiraError::Config(format!(
                "Configuration file not found at {}. Run 'harjira config init' to create one.",
                config_path.display()
            )));
        }

        let content = fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Override with environment variables if present
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Get the default configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let home = env::var("HOME").map_err(|_| {
            HarjiraError::Config("HOME environment variable not set".to_string())
        })?;

        let config_dir = PathBuf::from(home).join(".config").join("harjira");
        Ok(config_dir.join("config.toml"))
    }

    /// Create a template configuration file
    pub fn create_template() -> Result<()> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            return Err(HarjiraError::Config(format!(
                "Configuration file already exists at {}",
                config_path.display()
            )));
        }

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let template = r#"# Harjira Configuration File
# See: https://help.getharvest.com/api-v2/ for Harvest API docs
# See: https://developer.atlassian.com/cloud/jira/platform/rest/v3/ for Jira API docs

[harvest]
# Get your access token from: https://id.getharvest.com/developers
access_token = "your_harvest_access_token_here"
account_id = "your_account_id_here"
user_agent = "harjira (your.email@example.com)"

# Optional: Default project and task IDs for time entries
# Get these from: https://api.harvestapp.com/v2/projects
# project_id = 12345678
# task_id = 87654321

[jira]
# Create a Personal Access Token: https://id.atlassian.com/manage-profile/security/api-tokens
access_token = "your_jira_personal_access_token_here"
base_url = "https://your-company.atlassian.net"

[git]
# Leave empty to use current working directory
# Or specify paths to git repositories to monitor
repositories = []
# Example:
# repositories = [
#     "/home/user/projects/backend",
#     "/home/user/projects/frontend"
# ]

[settings]
# Skip prompts and automatically start timers (useful for systemd timer)
auto_start = false

# Skip prompts and automatically stop existing timers
auto_stop = false

# Automatically select ticket if only one is found
auto_select_single = true

[ticket_filter]
# Ignore specific ticket prefixes that match the pattern but aren't Jira tickets
# Common examples: CWE (Common Weakness Enumeration), CVE (Common Vulnerabilities)
denylist = ["CWE", "CVE"]

[ai]
# Enable AI-powered time entry generation
enabled = false

# AI provider: "openai" or "anthropic"
provider = "openai"

# API key for the AI provider
# OpenAI: Get from https://platform.openai.com/api-keys
# Anthropic: Get from https://console.anthropic.com/settings/keys
api_key = ""

# Optional: Specify model (defaults to provider's best model)
# model = "gpt-4o"  # or "claude-3-5-sonnet-20241022"

# Target hours per day (default: 8.0)
target_hours = 8.0
"#;

        fs::write(&config_path, template)?;

        // Set file permissions to 600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(token) = env::var("HARVEST_ACCESS_TOKEN") {
            self.harvest.access_token = token;
        }
        if let Ok(account_id) = env::var("HARVEST_ACCOUNT_ID") {
            self.harvest.account_id = account_id;
        }
        if let Ok(token) = env::var("JIRA_ACCESS_TOKEN") {
            self.jira.access_token = token;
        }
        if let Ok(base_url) = env::var("JIRA_BASE_URL") {
            self.jira.base_url = base_url;
        }
        if let Ok(enabled) = env::var("AI_ENABLED") {
            self.ai.enabled = enabled.parse().unwrap_or(false);
        }
        if let Ok(provider) = env::var("AI_PROVIDER") {
            self.ai.provider = provider;
        }
        if let Ok(api_key) = env::var("AI_API_KEY") {
            self.ai.api_key = api_key;
        }
        if let Ok(model) = env::var("AI_MODEL") {
            self.ai.model = Some(model);
        }
        if let Ok(target_hours) = env::var("AI_TARGET_HOURS") {
            if let Ok(hours) = target_hours.parse() {
                self.ai.target_hours = hours;
            }
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        if self.harvest.access_token.is_empty()
            || self.harvest.access_token.contains("your_harvest")
        {
            return Err(HarjiraError::Config(
                "Harvest access token not configured. Please update your config file."
                    .to_string(),
            ));
        }

        if self.harvest.account_id.is_empty() || self.harvest.account_id.contains("your_account")
        {
            return Err(HarjiraError::Config(
                "Harvest account ID not configured. Please update your config file.".to_string(),
            ));
        }

        if self.jira.access_token.is_empty() || self.jira.access_token.contains("your_jira") {
            return Err(HarjiraError::Config(
                "Jira access token not configured. Please update your config file.".to_string(),
            ));
        }

        if self.jira.base_url.is_empty() || self.jira.base_url.contains("your-company") {
            return Err(HarjiraError::Config(
                "Jira base URL not configured. Please update your config file.".to_string(),
            ));
        }

        if !self.jira.base_url.starts_with("http") {
            return Err(HarjiraError::Config(
                "Jira base URL must start with http:// or https://".to_string(),
            ));
        }

        // AI validation (only if enabled)
        if self.ai.enabled {
            if self.ai.api_key.is_empty() || self.ai.api_key.contains("your_") {
                return Err(HarjiraError::Config(
                    "AI is enabled but API key not configured. Please update your config file."
                        .to_string(),
                ));
            }

            if !["openai", "anthropic", "claude"]
                .contains(&self.ai.provider.to_lowercase().as_str())
            {
                return Err(HarjiraError::Config(format!(
                    "Unsupported AI provider: {}. Supported: openai, anthropic",
                    self.ai.provider
                )));
            }

            if self.ai.target_hours <= 0.0 || self.ai.target_hours > 24.0 {
                return Err(HarjiraError::Config(
                    "AI target_hours must be between 0 and 24".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Display current configuration (masking sensitive data)
    pub fn display(&self) {
        println!("Harvest Configuration:");
        println!("  Account ID: {}", self.harvest.account_id);
        println!(
            "  Access Token: {}***",
            &self.harvest.access_token.chars().take(8).collect::<String>()
        );
        println!("  User Agent: {}", self.harvest.user_agent);
        if let Some(project_id) = self.harvest.project_id {
            println!("  Default Project ID: {}", project_id);
        }
        if let Some(task_id) = self.harvest.task_id {
            println!("  Default Task ID: {}", task_id);
        }

        println!("\nJira Configuration:");
        println!("  Base URL: {}", self.jira.base_url);
        println!(
            "  Access Token: {}***",
            &self.jira.access_token.chars().take(8).collect::<String>()
        );

        println!("\nGit Configuration:");
        if self.git.repositories.is_empty() {
            println!("  Repositories: Using current working directory");
        } else {
            println!("  Repositories:");
            for repo in &self.git.repositories {
                println!("    - {}", repo);
            }
        }

        println!("\nSettings:");
        println!("  Auto-start timers: {}", self.settings.auto_start);
        println!("  Auto-stop timers: {}", self.settings.auto_stop);
        println!(
            "  Auto-select single ticket: {}",
            self.settings.auto_select_single
        );

        println!("\nAI Configuration:");
        println!("  Enabled: {}", self.ai.enabled);
        if self.ai.enabled {
            println!("  Provider: {}", self.ai.provider);
            if !self.ai.api_key.is_empty() {
                println!(
                    "  API Key: {}***",
                    &self.ai.api_key.chars().take(8).collect::<String>()
                );
            } else {
                println!("  API Key: (not set)");
            }
            if let Some(model) = &self.ai.model {
                println!("  Model: {}", model);
            }
            println!("  Target hours: {}", self.ai.target_hours);
        }
    }
}
