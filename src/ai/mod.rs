pub mod providers;

use crate::config::AiConfig;
use crate::error::{HarjiraError, Result};
use crate::models::{HarvestProject, HarvestTask, ProposedTimeEntry, TimeEntry};
use serde::Deserialize;

/// Context provided to AI for generating time entries
#[derive(Debug, Clone)]
pub struct AiContext {
    pub available_projects: Vec<HarvestProject>,
    pub available_tasks: Vec<HarvestTask>,
    pub existing_entries: Vec<TimeEntry>,
    pub target_hours: f64,
    pub today_total_hours: f64,
}

/// AI provider trait for extensibility
pub trait AiProvider: Send + Sync {
    fn generate_time_entries(
        &self,
        summary: &str,
        context: &AiContext,
    ) -> Result<Vec<ProposedTimeEntry>>;

    fn name(&self) -> &str;
}

/// Factory function to create the appropriate AI provider
pub fn create_provider(config: &AiConfig) -> Result<Box<dyn AiProvider>> {
    match config.provider.to_lowercase().as_str() {
        "openai" => Ok(Box::new(providers::openai::OpenAiProvider::new(
            config.api_key.clone(),
            config.model.clone(),
        )?)),
        "anthropic" | "claude" => Ok(Box::new(providers::anthropic::AnthropicProvider::new(
            config.api_key.clone(),
            config.model.clone(),
        )?)),
        _ => Err(HarjiraError::Config(format!(
            "Unsupported AI provider: {}. Supported: openai, anthropic",
            config.provider
        ))),
    }
}

/// Build the prompt to send to AI providers
pub fn build_prompt(summary: &str, context: &AiContext) -> String {
    let remaining_hours = context.target_hours - context.today_total_hours;

    let projects_json = serde_json::to_string_pretty(&context.available_projects)
        .unwrap_or_else(|_| "[]".to_string());

    let tasks_json = serde_json::to_string_pretty(&context.available_tasks)
        .unwrap_or_else(|_| "[]".to_string());

    let existing_entries_summary = if context.existing_entries.is_empty() {
        "No time entries logged yet today.".to_string()
    } else {
        let entries_list: Vec<String> = context
            .existing_entries
            .iter()
            .map(|e| {
                format!(
                    "- {:.2}h: {}",
                    e.hours.unwrap_or(0.0),
                    e.notes.as_deref().unwrap_or("No description")
                )
            })
            .collect();
        format!(
            "Already logged today ({:.2}h total):\n{}",
            context.today_total_hours,
            entries_list.join("\n")
        )
    };

    format!(
        r#"You are a time tracking assistant. Your task is to analyze a user's work summary
and generate time entries for Harvest.

USER'S WORK SUMMARY:
{summary}

CONTEXT:
- Target hours for today: {target_hours:.2}
- Already logged: {logged_hours:.2} hours
- Remaining to log: {remaining_hours:.2} hours

{existing_entries_summary}

AVAILABLE PROJECTS:
{projects_json}

AVAILABLE TASKS:
{tasks_json}

INSTRUCTIONS:
1. Parse the user's summary and identify distinct work activities
2. Allocate the remaining {remaining_hours:.2} hours across these activities
3. For each activity, select the most appropriate project_id and task_id from the lists above
4. Be reasonable with time allocation - don't create dozens of tiny entries
5. Aim for 2-5 entries typically, unless the user explicitly mentions more activities
6. Each entry should have clear, professional notes describing what was done
7. Hours should be in decimal format (e.g., 1.5 for 1 hour 30 minutes)
8. The sum of all entry hours should approximately equal {remaining_hours:.2} hours

IMPORTANT MATCHING RULES:
- Match project names based on keywords in the user's summary
- If uncertain about project/task, prefer general/administrative tasks
- If the user mentions specific project names, prioritize those
- Common task name mappings:
  * "Development" for coding/programming work
  * "Meeting" for meetings/calls
  * "Planning" for planning/design work
  * "Bug Fix" for debugging/fixing issues
  * "Code Review" for reviewing PRs
  * "Documentation" for writing docs

OUTPUT FORMAT (JSON):
Return a JSON object with a "time_entries" array. Each entry must have:
- "description": Clear description of the work (string)
- "project_id": Numeric project ID from the available projects (number)
- "task_id": Numeric task ID from the available tasks (number)
- "hours": Time in decimal hours (number)
- "confidence": Your confidence in this allocation from 0.0 to 1.0 (number, optional)

Example output:
{{
  "time_entries": [
    {{
      "description": "Implemented user authentication feature",
      "project_id": 12345,
      "task_id": 67890,
      "hours": 3.5,
      "confidence": 0.9
    }},
    {{
      "description": "Team standup meeting and sprint planning",
      "project_id": 12345,
      "task_id": 67891,
      "hours": 1.0,
      "confidence": 1.0
    }}
  ]
}}

Now generate the time entries based on the user's summary."#,
        summary = summary,
        target_hours = context.target_hours,
        logged_hours = context.today_total_hours,
        remaining_hours = remaining_hours,
        existing_entries_summary = existing_entries_summary,
        projects_json = projects_json,
        tasks_json = tasks_json,
    )
}

/// AI response structure
#[derive(Debug, Deserialize)]
struct AiResponse {
    time_entries: Vec<AiTimeEntry>,
}

/// AI time entry structure
#[derive(Debug, Deserialize)]
struct AiTimeEntry {
    description: String,
    project_id: u64,
    task_id: u64,
    hours: f64,
    confidence: Option<f64>,
}

/// Parse AI response JSON into proposed time entries
pub fn parse_response(response_text: &str) -> Result<Vec<ProposedTimeEntry>> {
    // Handle both raw JSON and JSON inside markdown code blocks
    let json_text = if response_text.contains("```json") {
        // Extract from markdown code block
        let start = response_text.find("```json").unwrap() + 7;
        let end = response_text[start..].find("```").unwrap() + start;
        &response_text[start..end]
    } else if response_text.contains("```") {
        let start = response_text.find("```").unwrap() + 3;
        let end = response_text[start..].find("```").unwrap() + start;
        &response_text[start..end]
    } else {
        response_text
    };

    let ai_response: AiResponse = serde_json::from_str(json_text.trim()).map_err(|e| {
        HarjiraError::Ai(format!(
            "Failed to parse AI response: {}. Raw response: {}",
            e,
            json_text.trim()
        ))
    })?;

    // Validate entries
    for entry in &ai_response.time_entries {
        if entry.hours <= 0.0 || entry.hours > 24.0 {
            return Err(HarjiraError::InvalidEntry(format!(
                "Invalid hours value: {}. Must be between 0 and 24.",
                entry.hours
            )));
        }
        if entry.description.trim().is_empty() {
            return Err(HarjiraError::InvalidEntry(
                "AI generated entry with empty description".to_string(),
            ));
        }
    }

    Ok(ai_response
        .time_entries
        .into_iter()
        .map(|e| ProposedTimeEntry {
            description: e.description,
            project_id: e.project_id,
            task_id: e.task_id,
            hours: e.hours,
            confidence_score: e.confidence,
        })
        .collect())
}
