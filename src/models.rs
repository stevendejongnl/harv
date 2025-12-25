use serde::{Deserialize, Serialize};

/// Represents a git commit
#[derive(Debug, Clone)]
pub struct Commit {
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Represents a Jira ticket
#[derive(Debug, Clone)]
pub struct Ticket {
    pub key: String,
    pub summary: String,
    pub status: Option<String>,
}

/// Harvest time entry request for creating a timer
#[derive(Debug, Serialize)]
pub struct CreateTimeEntryRequest {
    pub project_id: Option<u64>,
    pub task_id: Option<u64>,
    pub spent_date: String,
    pub notes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_reference: Option<ExternalReference>,
}

/// External reference to link Harvest entry to Jira
#[derive(Debug, Serialize)]
pub struct ExternalReference {
    pub id: String,
    pub group_id: String,
    pub permalink: String,
}

/// Harvest time entry response
#[derive(Debug, Deserialize, Clone)]
pub struct TimeEntry {
    pub id: u64,
    pub spent_date: String,
    pub hours: Option<f64>,
    pub notes: Option<String>,
    pub is_running: bool,
    pub project: Option<ProjectInfo>,
    pub task: Option<TaskInfo>,
    pub started_time: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProjectInfo {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaskInfo {
    pub id: u64,
    pub name: String,
}

/// Response from Harvest API for time entries list
#[derive(Debug, Deserialize)]
pub struct TimeEntriesResponse {
    pub time_entries: Vec<TimeEntry>,
}

/// Jira issue response
#[derive(Debug, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraFields {
    pub summary: String,
    pub status: JiraStatus,
}

#[derive(Debug, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

/// Application context for passing configuration and flags
#[derive(Debug, Clone)]
pub struct Context {
    pub dry_run: bool,
    pub auto_start: bool,
    pub auto_stop: bool,
    pub quiet: bool,
    pub verbose: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            dry_run: false,
            auto_start: false,
            auto_stop: false,
            quiet: false,
            verbose: false,
        }
    }
}

/// Proposed time entry from AI provider
#[derive(Debug, Clone)]
pub struct ProposedTimeEntry {
    pub description: String,
    pub project_id: u64,
    pub task_id: u64,
    pub hours: f64,
    pub confidence_score: Option<f64>,
}

/// Request for creating a stopped time entry (not a running timer)
#[derive(Debug, Serialize)]
pub struct CreateStoppedTimeEntryRequest {
    pub project_id: u64,
    pub task_id: u64,
    pub spent_date: String,
    pub notes: String,
    pub hours: f64,
}

/// Response from Harvest API for projects list
#[derive(Debug, Deserialize)]
pub struct ProjectsResponse {
    pub projects: Vec<HarvestProject>,
}

/// Harvest project information
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct HarvestProject {
    pub id: u64,
    pub name: String,
    pub code: Option<String>,
}

/// Response from Harvest API for task assignments
#[derive(Debug, Deserialize)]
pub struct TaskAssignmentsResponse {
    pub task_assignments: Vec<TaskAssignment>,
}

/// Task assignment with activation status
#[derive(Debug, Deserialize)]
pub struct TaskAssignment {
    pub is_active: bool,
    pub task: TaskDetail,
}

/// Detailed task information from Harvest
#[derive(Debug, Deserialize)]
pub struct TaskDetail {
    pub id: u64,
    pub name: String,
}

/// Simplified task information
#[derive(Debug, Clone, Serialize)]
pub struct HarvestTask {
    pub id: u64,
    pub name: String,
}

/// Response from /v2/users/me/project_assignments
#[derive(Debug, Deserialize)]
pub struct UserProjectAssignmentsResponse {
    pub project_assignments: Vec<UserProjectAssignment>,
}

/// Project assignment for a user
#[derive(Debug, Deserialize)]
pub struct UserProjectAssignment {
    pub id: u64,
    pub is_active: bool,
    pub project: HarvestProject,
    pub task_assignments: Vec<TaskAssignment>,
}
