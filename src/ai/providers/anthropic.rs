use crate::ai::{build_prompt, parse_response, AiContext, AiProvider};
use crate::error::{HarjiraError, Result};
use crate::models::ProposedTimeEntry;
use log::debug;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        if api_key.is_empty() {
            return Err(HarjiraError::Config(
                "Anthropic API key is required".to_string(),
            ));
        }

        let client = Client::new();
        let model = model.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

impl AiProvider for AnthropicProvider {
    fn generate_time_entries(
        &self,
        summary: &str,
        context: &AiContext,
    ) -> Result<Vec<ProposedTimeEntry>> {
        let prompt = build_prompt(summary, context);

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let url = "https://api.anthropic.com/v1/messages";
        debug!("POST {}", url);

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| HarjiraError::Ai(format!("Anthropic API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Ai(format!(
                "Anthropic API error ({}): {}",
                status, error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response.json().map_err(|e| {
            HarjiraError::Ai(format!("Failed to parse Anthropic response: {}", e))
        })?;

        if anthropic_response.content.is_empty() {
            return Err(HarjiraError::Ai(
                "Anthropic returned no content".to_string(),
            ));
        }

        let content = &anthropic_response.content[0].text;
        debug!("Anthropic response: {}", content);

        parse_response(content)
    }

    fn name(&self) -> &str {
        "Anthropic Claude"
    }
}
