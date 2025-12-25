use crate::config::HarvestConfig;
use crate::error::{HarjiraError, Result};
use crate::models::{
    Context, CreateStoppedTimeEntryRequest, CreateTimeEntryRequest, ExternalReference,
    HarvestProject, HarvestTask, ProjectsResponse, TaskAssignmentsResponse, TimeEntriesResponse,
    TimeEntry, UserProjectAssignmentsResponse,
};
use chrono::Local;
use log::{debug, info, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;

pub struct HarvestClient {
    client: Client,
    base_url: String,
    config: HarvestConfig,
}

impl HarvestClient {
    pub fn new(config: HarvestConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();

        // Authorization: Bearer {token}
        let auth_value = format!("Bearer {}", config.access_token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).map_err(|e| {
                HarjiraError::Config(format!("Invalid Harvest access token: {}", e))
            })?,
        );

        // Harvest-Account-Id
        headers.insert(
            "Harvest-Account-Id",
            HeaderValue::from_str(&config.account_id).map_err(|e| {
                HarjiraError::Config(format!("Invalid Harvest account ID: {}", e))
            })?,
        );

        // User-Agent
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.user_agent).map_err(|e| {
                HarjiraError::Config(format!("Invalid user agent: {}", e))
            })?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| HarjiraError::Harvest(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: "https://api.harvestapp.com/v2".to_string(),
            config,
        })
    }

    /// Get all time entries for today
    pub fn get_todays_time_entries(&self) -> Result<Vec<TimeEntry>> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let url = format!(
            "{}/time_entries?from={}&to={}",
            self.base_url, today, today
        );

        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| HarjiraError::Harvest(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let entries_response: TimeEntriesResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse time entries response: {}", e))
        })?;

        debug!(
            "Retrieved {} time entries for today",
            entries_response.time_entries.len()
        );

        Ok(entries_response.time_entries)
    }

    /// Get the currently running time entry, if any
    pub fn get_running_timer(&self) -> Result<Option<TimeEntry>> {
        let entries = self.get_todays_time_entries()?;
        Ok(entries.into_iter().find(|e| e.is_running))
    }

    /// Create a new time entry (start a timer)
    pub fn create_time_entry(
        &self,
        jira_ticket: &str,
        description: &str,
        jira_url: &str,
        ctx: &Context,
    ) -> Result<TimeEntry> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let notes = format!("{} - {}", jira_ticket, description);

        let request = CreateTimeEntryRequest {
            project_id: self.config.project_id,
            task_id: self.config.task_id,
            spent_date: today,
            notes: notes.clone(),
            external_reference: Some(ExternalReference {
                id: jira_ticket.to_string(),
                group_id: "jira".to_string(),
                permalink: jira_url.to_string(),
            }),
        };

        if ctx.dry_run {
            info!("[DRY RUN] Would create time entry:");
            info!("  Project ID: {:?}", request.project_id);
            info!("  Task ID: {:?}", request.task_id);
            info!("  Notes: {}", request.notes);
            info!("  External Reference: {}", jira_url);
            return Ok(TimeEntry {
                id: 0,
                spent_date: request.spent_date,
                hours: Some(0.0),
                notes: Some(request.notes),
                is_running: true,
                project: None,
                task: None,
                started_time: None,
            });
        }

        let url = format!("{}/time_entries", self.base_url);
        debug!("POST {}", url);
        debug!("Request body: {:?}", request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| HarjiraError::Harvest(format!("Failed to create time entry: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to create time entry ({}): {}",
                status, error_text
            )));
        }

        let entry: TimeEntry = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse created time entry: {}", e))
        })?;

        info!("Created time entry: {}", notes);
        Ok(entry)
    }

    /// Stop a running timer
    pub fn stop_time_entry(&self, entry_id: u64, ctx: &Context) -> Result<TimeEntry> {
        if ctx.dry_run {
            info!("[DRY RUN] Would stop time entry {}", entry_id);
            return Ok(TimeEntry {
                id: entry_id,
                spent_date: Local::now().format("%Y-%m-%d").to_string(),
                hours: Some(0.0),
                notes: None,
                is_running: false,
                project: None,
                task: None,
                started_time: None,
            });
        }

        let url = format!("{}/time_entries/{}/stop", self.base_url, entry_id);
        debug!("PATCH {}", url);

        let response = self.client.patch(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to stop time entry: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to stop time entry ({}): {}",
                status, error_text
            )));
        }

        let entry: TimeEntry = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse stopped time entry: {}", e))
        })?;

        info!("Stopped time entry {}", entry_id);
        Ok(entry)
    }

    /// Calculate total hours logged today
    pub fn get_total_hours_today(&self) -> Result<f64> {
        let entries = self.get_todays_time_entries()?;
        let total = entries.iter().filter_map(|e| e.hours).sum();
        Ok(total)
    }

    /// Get all active projects accessible to the user
    pub fn get_projects(&self) -> Result<Vec<HarvestProject>> {
        let url = format!("{}/projects?is_active=true", self.base_url);

        debug!("GET {}", url);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch projects: {}", e))
        })?;

        // If 403 Forbidden, fall back to user project assignments
        if response.status() == StatusCode::FORBIDDEN {
            warn!("Access denied to /v2/projects endpoint. Falling back to user project assignments.");
            warn!("This is normal for Personal Access Tokens with limited permissions.");
            return self.get_user_project_assignments();
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch projects ({}): {}",
                status, error_text
            )));
        }

        let projects_response: ProjectsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse projects response: {}", e))
        })?;

        debug!(
            "Retrieved {} projects",
            projects_response.projects.len()
        );

        Ok(projects_response.projects)
    }

    /// Fallback method to get projects via user assignments (requires only user permissions)
    fn get_user_project_assignments(&self) -> Result<Vec<HarvestProject>> {
        let url = format!("{}/users/me/project_assignments", self.base_url);
        debug!("GET {} (fallback method)", url);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch user project assignments: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch user project assignments ({}): {}",
                status, error_text
            )));
        }

        let assignments_response: UserProjectAssignmentsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!(
                "Failed to parse user project assignments response: {}",
                e
            ))
        })?;

        let projects: Vec<HarvestProject> = assignments_response
            .project_assignments
            .into_iter()
            .filter(|pa| pa.is_active)
            .map(|pa| pa.project)
            .collect();

        debug!("Retrieved {} projects via user assignments", projects.len());
        Ok(projects)
    }

    /// Get available tasks for a specific project
    pub fn get_project_tasks(&self, project_id: u64) -> Result<Vec<HarvestTask>> {
        let url = format!(
            "{}/projects/{}/task_assignments",
            self.base_url, project_id
        );

        debug!("GET {}", url);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch tasks: {}", e))
        })?;

        // If 403 Forbidden, try to get tasks from user assignments
        if response.status() == StatusCode::FORBIDDEN {
            warn!(
                "Access denied to /v2/projects/{}/task_assignments. Trying user assignments.",
                project_id
            );
            return self.get_user_project_tasks(project_id);
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch tasks ({}): {}",
                status, error_text
            )));
        }

        let tasks_response: TaskAssignmentsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse tasks response: {}", e))
        })?;

        let tasks: Vec<HarvestTask> = tasks_response
            .task_assignments
            .into_iter()
            .filter(|ta| ta.is_active)
            .map(|ta| HarvestTask {
                id: ta.task.id,
                name: ta.task.name,
            })
            .collect();

        debug!("Retrieved {} tasks for project {}", tasks.len(), project_id);

        Ok(tasks)
    }

    /// Fallback method to get tasks from user project assignments
    fn get_user_project_tasks(&self, project_id: u64) -> Result<Vec<HarvestTask>> {
        let url = format!("{}/users/me/project_assignments", self.base_url);
        debug!("GET {} (to fetch tasks for project {})", url, project_id);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch user project assignments: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch user project assignments ({}): {}",
                status, error_text
            )));
        }

        let assignments_response: UserProjectAssignmentsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!(
                "Failed to parse user project assignments response: {}",
                e
            ))
        })?;

        // Find the specific project assignment
        let project_assignment = assignments_response
            .project_assignments
            .into_iter()
            .find(|pa| pa.is_active && pa.project.id == project_id)
            .ok_or_else(|| {
                HarjiraError::Harvest(format!(
                    "Project {} not found in user assignments or not accessible",
                    project_id
                ))
            })?;

        // Extract tasks from the project assignment
        let tasks: Vec<HarvestTask> = project_assignment
            .task_assignments
            .into_iter()
            .filter(|ta| ta.is_active)
            .map(|ta| HarvestTask {
                id: ta.task.id,
                name: ta.task.name,
            })
            .collect();

        debug!(
            "Retrieved {} tasks for project {} via user assignments",
            tasks.len(),
            project_id
        );
        Ok(tasks)
    }

    /// Get all available tasks across all projects
    /// Optimized to use a single API call when using limited permissions
    pub fn get_all_available_tasks(&self) -> Result<Vec<(u64, HarvestTask)>> {
        // Try direct projects endpoint first
        let url = format!("{}/projects?is_active=true", self.base_url);
        debug!("GET {}", url);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch projects: {}", e))
        })?;

        // If 403 Forbidden, use optimized user assignments path
        if response.status() == StatusCode::FORBIDDEN {
            debug!("Access denied to /v2/projects. Using optimized user assignments fetch.");
            return self.get_all_tasks_from_user_assignments();
        }

        // If we have full access, fetch projects then tasks individually
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch projects ({}): {}",
                status, error_text
            )));
        }

        let projects_response: ProjectsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse projects response: {}", e))
        })?;

        let mut all_tasks = Vec::new();
        for project in projects_response.projects {
            match self.get_project_tasks(project.id) {
                Ok(tasks) => {
                    for task in tasks {
                        all_tasks.push((project.id, task));
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch tasks for project {}: {}", project.id, e);
                    // Continue with other projects (non-fatal)
                }
            }
        }

        debug!("Retrieved {} total task assignments", all_tasks.len());
        Ok(all_tasks)
    }

    /// Optimized method to get all projects and tasks in a single API call
    /// Used when PAT has limited permissions
    fn get_all_tasks_from_user_assignments(&self) -> Result<Vec<(u64, HarvestTask)>> {
        let url = format!("{}/users/me/project_assignments", self.base_url);
        debug!("GET {} (optimized - fetching all projects and tasks)", url);

        let response = self.client.get(&url).send().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to fetch user project assignments: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to fetch user project assignments ({}): {}",
                status, error_text
            )));
        }

        let assignments_response: UserProjectAssignmentsResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!(
                "Failed to parse user project assignments response: {}",
                e
            ))
        })?;

        let mut all_tasks = Vec::new();
        for assignment in assignments_response.project_assignments {
            if !assignment.is_active {
                continue;
            }

            let project_id = assignment.project.id;
            for task_assignment in assignment.task_assignments {
                if task_assignment.is_active {
                    all_tasks.push((
                        project_id,
                        HarvestTask {
                            id: task_assignment.task.id,
                            name: task_assignment.task.name,
                        },
                    ));
                }
            }
        }

        debug!(
            "Retrieved {} total task assignments via optimized path",
            all_tasks.len()
        );
        Ok(all_tasks)
    }

    /// Calculate remaining hours needed to reach target
    pub fn calculate_remaining_hours(&self, target_hours: f64) -> Result<f64> {
        let total = self.get_total_hours_today()?;
        let remaining = (target_hours - total).max(0.0);
        Ok(remaining)
    }

    /// Create a stopped time entry (not a running timer)
    pub fn create_stopped_time_entry(
        &self,
        description: &str,
        project_id: u64,
        task_id: u64,
        hours: f64,
        ctx: &Context,
    ) -> Result<TimeEntry> {
        let today = Local::now().format("%Y-%m-%d").to_string();

        let request = CreateStoppedTimeEntryRequest {
            project_id,
            task_id,
            spent_date: today.clone(),
            notes: description.to_string(),
            hours,
        };

        if ctx.dry_run {
            info!("[DRY RUN] Would create stopped time entry:");
            info!("  Project ID: {}", request.project_id);
            info!("  Task ID: {}", request.task_id);
            info!("  Notes: {}", request.notes);
            info!("  Hours: {}", request.hours);
            return Ok(TimeEntry {
                id: 0,
                spent_date: request.spent_date,
                hours: Some(request.hours),
                notes: Some(request.notes),
                is_running: false,
                project: None,
                task: None,
                started_time: None,
            });
        }

        let url = format!("{}/time_entries", self.base_url);
        debug!("POST {}", url);
        debug!("Request body: {:?}", request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| {
                HarjiraError::Harvest(format!("Failed to create time entry: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to create time entry ({}): {}",
                status, error_text
            )));
        }

        let entry: TimeEntry = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse created time entry: {}", e))
        })?;

        info!("Created time entry: {} ({:.2}h)", description, hours);
        Ok(entry)
    }

    /// Create a new time entry with custom date (start a timer)
    pub fn create_time_entry_with_date(
        &self,
        description: &str,
        project_id: u64,
        task_id: u64,
        spent_date: &str,
        ctx: &Context,
    ) -> Result<TimeEntry> {
        let request = CreateTimeEntryRequest {
            project_id: Some(project_id),
            task_id: Some(task_id),
            spent_date: spent_date.to_string(),
            notes: description.to_string(),
            external_reference: None,
        };

        if ctx.dry_run {
            info!("[DRY RUN] Would create time entry:");
            info!("  Project ID: {}", project_id);
            info!("  Task ID: {}", task_id);
            info!("  Date: {}", spent_date);
            info!("  Notes: {}", description);
            return Ok(TimeEntry {
                id: 0,
                spent_date: spent_date.to_string(),
                hours: Some(0.0),
                notes: Some(description.to_string()),
                is_running: true,
                project: None,
                task: None,
                started_time: None,
            });
        }

        let url = format!("{}/time_entries", self.base_url);
        debug!("POST {}", url);
        debug!("Request body: {:?}", request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| HarjiraError::Harvest(format!("Failed to create time entry: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to create time entry ({}): {}",
                status, error_text
            )));
        }

        let entry: TimeEntry = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse created time entry: {}", e))
        })?;

        info!("Created time entry: {} on {}", description, spent_date);
        Ok(entry)
    }

    /// Create a stopped time entry with custom date
    pub fn create_stopped_time_entry_with_date(
        &self,
        description: &str,
        project_id: u64,
        task_id: u64,
        hours: f64,
        spent_date: &str,
        ctx: &Context,
    ) -> Result<TimeEntry> {
        let request = CreateStoppedTimeEntryRequest {
            project_id,
            task_id,
            spent_date: spent_date.to_string(),
            notes: description.to_string(),
            hours,
        };

        if ctx.dry_run {
            info!("[DRY RUN] Would create stopped time entry:");
            info!("  Project ID: {}", project_id);
            info!("  Task ID: {}", task_id);
            info!("  Date: {}", spent_date);
            info!("  Notes: {}", description);
            info!("  Hours: {}", hours);
            return Ok(TimeEntry {
                id: 0,
                spent_date: spent_date.to_string(),
                hours: Some(hours),
                notes: Some(description.to_string()),
                is_running: false,
                project: None,
                task: None,
                started_time: None,
            });
        }

        let url = format!("{}/time_entries", self.base_url);
        debug!("POST {}", url);
        debug!("Request body: {:?}", request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| {
                HarjiraError::Harvest(format!("Failed to create time entry: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "Failed to create time entry ({}): {}",
                status, error_text
            )));
        }

        let entry: TimeEntry = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse created time entry: {}", e))
        })?;

        info!(
            "Created time entry: {} ({:.2}h) on {}",
            description, hours, spent_date
        );
        Ok(entry)
    }

    /// Get total hours logged for a specific date
    pub fn get_total_hours_for_date(&self, date: &str) -> Result<f64> {
        let url = format!("{}/time_entries?from={}&to={}", self.base_url, date, date);

        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| HarjiraError::Harvest(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HarjiraError::Harvest(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let entries_response: TimeEntriesResponse = response.json().map_err(|e| {
            HarjiraError::Harvest(format!("Failed to parse time entries response: {}", e))
        })?;

        let total = entries_response
            .time_entries
            .iter()
            .filter_map(|e| e.hours)
            .sum();

        Ok(total)
    }
}
