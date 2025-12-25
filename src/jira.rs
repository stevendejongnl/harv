use crate::config::JiraConfig;
use crate::error::{HarjiraError, Result};
use crate::models::{JiraIssue, Ticket};
use log::{debug, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

pub struct JiraClient {
    client: Client,
    config: JiraConfig,
}

impl JiraClient {
    pub fn new(config: JiraConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();

        // Authorization: Bearer {token}
        let auth_value = format!("Bearer {}", config.access_token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).map_err(|e| {
                HarjiraError::Config(format!("Invalid Jira access token: {}", e))
            })?,
        );

        // Content-Type: application/json
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| HarjiraError::Jira(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Get issue details from Jira
    pub fn get_issue(&self, ticket_key: &str) -> Result<Ticket> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            self.config.base_url.trim_end_matches('/'),
            ticket_key
        );

        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| HarjiraError::Jira(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Common error cases
            if status == 404 {
                return Err(HarjiraError::Jira(format!(
                    "Ticket {} not found. Verify the ticket key is correct.",
                    ticket_key
                )));
            } else if status == 401 {
                return Err(HarjiraError::Jira(
                    "Authentication failed. Check your Jira access token.".to_string(),
                ));
            } else if status == 403 {
                return Err(HarjiraError::Jira(format!(
                    "Access denied to ticket {}. Check your permissions.",
                    ticket_key
                )));
            }

            return Err(HarjiraError::Jira(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let issue: JiraIssue = response
            .json()
            .map_err(|e| HarjiraError::Jira(format!("Failed to parse issue response: {}", e)))?;

        debug!(
            "Retrieved Jira issue: {} - {}",
            issue.key, issue.fields.summary
        );

        Ok(Ticket {
            key: issue.key,
            summary: issue.fields.summary,
            status: Some(issue.fields.status.name),
        })
    }

    /// Get multiple issues at once
    pub fn get_issues(&self, ticket_keys: &[String]) -> Vec<Ticket> {
        let mut tickets = Vec::new();

        for key in ticket_keys {
            match self.get_issue(key) {
                Ok(ticket) => tickets.push(ticket),
                Err(e) => {
                    warn!("Failed to fetch Jira ticket {}: {}", key, e);
                    // Create a ticket with just the key for failed fetches
                    tickets.push(Ticket {
                        key: key.clone(),
                        summary: format!("(Failed to fetch: {})", e),
                        status: None,
                    });
                }
            }
        }

        tickets
    }

    /// Build the Jira ticket URL
    pub fn get_ticket_url(&self, ticket_key: &str) -> String {
        format!(
            "{}/browse/{}",
            self.config.base_url.trim_end_matches('/'),
            ticket_key
        )
    }
}
