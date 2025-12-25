use crate::ai::{build_prompt, parse_response, AiContext, AiProvider};
use crate::error::{HarjiraError, Result};
use crate::models::ProposedTimeEntry;
use log::debug;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        if api_key.is_empty() {
            return Err(HarjiraError::Config(
                "OpenAI API key is required".to_string(),
            ));
        }

        let client = Client::new();
        let model = model.unwrap_or_else(|| "gpt-4o".to_string());

        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

impl AiProvider for OpenAiProvider {
    fn generate_time_entries(
        &self,
        summary: &str,
        context: &AiContext,
    ) -> Result<Vec<ProposedTimeEntry>> {
        let prompt = build_prompt(summary, context);

        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let url = "https://api.openai.com/v1/chat/completions";
        debug!("POST {}", url);

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| HarjiraError::Ai(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Ai(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let openai_response: OpenAiResponse = response.json().map_err(|e| {
            HarjiraError::Ai(format!("Failed to parse OpenAI response: {}", e))
        })?;

        if openai_response.choices.is_empty() {
            return Err(HarjiraError::Ai(
                "OpenAI returned no choices".to_string(),
            ));
        }

        let content = &openai_response.choices[0].message.content;
        debug!("OpenAI response: {}", content);

        parse_response(content)
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}
