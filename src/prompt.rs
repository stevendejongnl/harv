use crate::error::{HarjiraError, Result};
use crate::models::{EntryType, HarvestProject, HarvestTask, ProposedTimeEntry, Ticket, TimeEntry};
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Editor, FuzzySelect, Input, MultiSelect, Select};

/// Prompt user to select a Jira ticket from multiple options
pub fn prompt_ticket_selection(tickets: &[Ticket]) -> Result<Ticket> {
    if tickets.is_empty() {
        return Err(HarjiraError::NoTicketsFound);
    }

    // Build display items
    let items: Vec<String> = tickets
        .iter()
        .map(|t| {
            let status_str = t
                .status
                .as_ref()
                .map(|s| format!(" [{}]", s))
                .unwrap_or_default();
            format!("{} - {}{}", t.key, t.summary, status_str)
        })
        .collect();

    println!("\nMultiple Jira tickets found in today's commits:");

    let selection = Select::new()
        .with_prompt("Select a ticket to track")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(tickets[selection].clone())
}

/// Confirm whether to stop the current timer and start a new one
pub fn confirm_stop_timer(current_timer: &TimeEntry, new_ticket: &str) -> Result<bool> {
    let current_notes = current_timer
        .notes
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("Unknown");

    let project_info = current_timer
        .project
        .as_ref()
        .map(|p| format!(" ({})", p.name))
        .unwrap_or_default();

    println!("\n⚠️  Timer currently running:");
    println!("   {}{}", current_notes, project_info);

    if let Some(started) = &current_timer.started_time {
        println!("   Started at: {}", started);
    }

    if let Some(hours) = current_timer.hours {
        println!("   Duration: {:.2} hours", hours);
    }

    println!("\nNew ticket: {}", new_ticket);

    Confirm::new()
        .with_prompt("Stop current timer and start new one?")
        .default(false)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)
}

/// Display a success message
pub fn display_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), style(message).green());
}

/// Display an info message
pub fn display_info(message: &str) {
    println!("{} {}", style("ℹ").cyan().bold(), style(message).cyan());
}

/// Display a warning message
pub fn display_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
}

/// Prompt user to enter their work summary
pub fn prompt_work_summary() -> Result<String> {
    println!("\nEnter a summary of your work today:");
    println!("(You can describe multiple activities)");
    println!();

    let summary = Editor::new()
        .edit("Enter your work summary here...\n")
        .map_err(|_| HarjiraError::UserCancelled)?
        .ok_or_else(|| HarjiraError::UserCancelled)?;

    Ok(summary)
}

/// Display proposed entries and allow user to review/edit
pub fn review_and_approve_entries(
    entries: &[ProposedTimeEntry],
    projects: &[HarvestProject],
) -> Result<Vec<ProposedTimeEntry>> {
    println!("\n{}", style("=".repeat(80)).cyan().bold());
    println!("{}", style("AI Generated Time Entries").cyan().bold());
    println!("{}", style("=".repeat(80)).cyan().bold());

    let total_hours: f64 = entries.iter().map(|e| e.hours).sum();

    // Build items for display and selection (plain text, colors will come from theme)
    let items: Vec<String> = entries
        .iter()
        .map(|entry| {
            let project_name = projects
                .iter()
                .find(|p| p.id == entry.project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown Project");

            let confidence_str = if let Some(conf) = entry.confidence_score {
                format!(" [confidence: {:.0}%]", conf * 100.0)
            } else {
                String::new()
            };

            format!(
                "{:.2}h - {} ({}){} ",
                entry.hours,
                entry.description,
                project_name,
                confidence_str
            )
        })
        .collect();

    println!();
    println!("{} {}",
        style("Total:").yellow().bold(),
        style(format!("{:.2} hours", total_hours)).yellow().bold()
    );
    println!();

    // Multi-select for approval with colorful theme
    let defaults = vec![true; entries.len()];

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select entries to create (Space=toggle, Enter=confirm, Ctrl+C=cancel)")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    if selections.is_empty() {
        return Ok(Vec::new());
    }

    let mut approved: Vec<ProposedTimeEntry> = selections
        .iter()
        .map(|&idx| entries[idx].clone())
        .collect();

    // Ask if user wants to edit any entries
    println!();
    let want_edit = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to edit any entries? (hours/description)")
        .default(false)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    if want_edit {
        // Build list of entries to select for editing
        let edit_items: Vec<String> = approved
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                format!(
                    "{}. {:.2}h - {}",
                    idx + 1,
                    entry.hours,
                    entry.description
                )
            })
            .collect();

        let edit_selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select entries to edit")
            .items(&edit_items)
            .interact()
            .map_err(|_| HarjiraError::UserCancelled)?;

        // Edit each selected entry
        for &idx in &edit_selections {
            let entry = &mut approved[idx];

            println!();
            println!("{}", style(format!("Editing entry {}", idx + 1)).cyan().bold());
            println!("{}", style("=".repeat(60)).cyan());

            // Edit hours
            let hours_str: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Hours (e.g., 1.5 or 1:30)")
                .default(format!("{:.2}", entry.hours))
                .validate_with(|input: &String| -> std::result::Result<(), String> {
                    match crate::time_parser::parse_hours(input) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e.to_string()),
                    }
                })
                .interact_text()
                .map_err(|_| HarjiraError::UserCancelled)?;

            let new_hours = crate::time_parser::parse_hours(&hours_str)?;

            // Edit description
            let new_description: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Description")
                .default(entry.description.clone())
                .interact_text()
                .map_err(|_| HarjiraError::UserCancelled)?;

            entry.hours = new_hours;
            entry.description = new_description;

            println!("{}", style("✓ Entry updated").green());
        }
    }

    // Confirm final entries
    let approved_total: f64 = approved.iter().map(|e| e.hours).sum();
    println!();
    println!("{}", style("=".repeat(80)).cyan().bold());
    println!("{}", style("Final entries to create:").cyan().bold());
    for (idx, entry) in approved.iter().enumerate() {
        println!(
            "  {}. {} - {}",
            style(idx + 1).cyan().bold(),
            style(format!("{:.2}h", entry.hours)).green().bold(),
            style(&entry.description).white()
        );
    }
    println!();
    println!(
        "{} {} {} {} {}",
        style("Will create").white(),
        style(approved.len()).green().bold(),
        style("entries totaling").white(),
        style(format!("{:.2}", approved_total)).green().bold(),
        style("hours").white()
    );
    println!("{}", style("=".repeat(80)).cyan().bold());

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with creation?")
        .default(true)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    if !confirmed {
        return Ok(Vec::new());
    }

    Ok(approved)
}

/// Prompt user to select entry type (running timer vs stopped entry)
pub fn prompt_entry_type() -> Result<EntryType> {
    let items = vec![
        "Running timer (start now, stop later)",
        "Stopped entry (specify hours worked)",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What type of entry would you like to create?")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    match selection {
        0 => Ok(EntryType::Running),
        1 => Ok(EntryType::Stopped),
        _ => unreachable!(),
    }
}

/// Prompt user to select a date
pub fn prompt_date_selection() -> Result<String> {
    use chrono::{Duration, Local};

    let today = Local::now().date_naive();

    // Build list of recent dates
    let mut items = Vec::new();
    items.push(format!("Today ({})", today.format("%Y-%m-%d")));
    items.push(format!(
        "Yesterday ({})",
        (today - Duration::days(1)).format("%Y-%m-%d")
    ));

    for i in 2..=6 {
        let date = today - Duration::days(i);
        items.push(format!("{} days ago ({})", i, date.format("%Y-%m-%d")));
    }

    items.push("Custom date...".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select date for time entry")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    if selection == 7 {
        // Custom date input
        prompt_custom_date()
    } else {
        // Extract date from selected item
        let date = today - Duration::days(selection as i64);
        Ok(date.format("%Y-%m-%d").to_string())
    }
}

/// Prompt for custom date input with validation
fn prompt_custom_date() -> Result<String> {
    use chrono::{Duration, Local, NaiveDate};

    let today = Local::now().date_naive();
    let min_date = today - Duration::days(90); // 90 days back limit

    let date_str: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter date (YYYY-MM-DD)")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            match NaiveDate::parse_from_str(input, "%Y-%m-%d") {
                Ok(date) => {
                    if date > today {
                        Err("Date cannot be in the future")
                    } else if date < min_date {
                        Err("Date must be within the last 90 days")
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err("Invalid date format. Use YYYY-MM-DD (e.g., 2025-12-24)"),
            }
        })
        .interact_text()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(date_str)
}

/// Prompt user to select a project
pub fn prompt_project_selection(projects: &[HarvestProject]) -> Result<HarvestProject> {
    if projects.is_empty() {
        return Err(HarjiraError::Config(
            "No active projects found in your Harvest account".to_string(),
        ));
    }

    let items: Vec<String> = projects
        .iter()
        .map(|p| {
            if let Some(code) = &p.code {
                format!("{} [{}]", p.name, code)
            } else {
                p.name.clone()
            }
        })
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select project (type to search)")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(projects[selection].clone())
}

/// Prompt user to select a task
pub fn prompt_task_selection(tasks: &[HarvestTask]) -> Result<HarvestTask> {
    if tasks.is_empty() {
        return Err(HarjiraError::Config(
            "No tasks available for the selected project".to_string(),
        ));
    }

    let items: Vec<String> = tasks.iter().map(|t| t.name.clone()).collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select task (type to search)")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(tasks[selection].clone())
}

/// Prompt for time entry description
pub fn prompt_description() -> Result<String> {
    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter description")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("Description cannot be empty")
            } else if input.len() > 500 {
                Err("Description too long (max 500 characters)")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(description.trim().to_string())
}

/// Prompt for hours with validation
pub fn prompt_hours() -> Result<f64> {
    let hours_str: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter hours (e.g., 1.5 or 1:30)")
        .validate_with(|input: &String| -> std::result::Result<(), String> {
            match crate::time_parser::parse_hours(input) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        })
        .interact_text()
        .map_err(|_| HarjiraError::UserCancelled)?;

    let hours = crate::time_parser::parse_hours(&hours_str)?;

    Ok(hours)
}

/// Confirm entry creation with full details
pub fn confirm_entry_creation(
    entry_type: &EntryType,
    date: &str,
    project: &str,
    task: &str,
    description: &str,
    hours: Option<f64>,
) -> Result<bool> {
    println!();
    println!("{}", style("=".repeat(60)).cyan().bold());
    println!("{}", style("Entry Summary").cyan().bold());
    println!("{}", style("=".repeat(60)).cyan().bold());
    println!(
        "Type:        {}",
        match entry_type {
            EntryType::Running => style("Running Timer").green(),
            EntryType::Stopped => style("Stopped Entry").yellow(),
        }
    );
    println!("Date:        {}", style(date).white());
    println!("Project:     {}", style(project).white());
    println!("Task:        {}", style(task).white());
    println!("Description: {}", style(description).white());
    if let Some(h) = hours {
        println!("Hours:       {}", style(format!("{:.2}h", h)).green().bold());
    }
    println!("{}", style("=".repeat(60)).cyan().bold());
    println!();

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create this entry?")
        .default(true)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)
}

/// Confirm stopping existing timer for new manual entry
pub fn confirm_stop_timer_for_new(current_timer: &TimeEntry) -> Result<bool> {
    let current_notes = current_timer
        .notes
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("Unknown");

    println!(
        "\n{}",
        style("⚠ Timer currently running:").yellow().bold()
    );
    println!("   {}", current_notes);

    if let Some(hours) = current_timer.hours {
        println!("   Duration: {:.2} hours", hours);
    }

    println!();

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Stop current timer to create new entry?")
        .default(false)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)
}

/// Prompt user to select a time entry from a list
pub fn prompt_entry_selection(entries: &[TimeEntry]) -> Result<&TimeEntry> {
    if entries.is_empty() {
        return Err(HarjiraError::Harvest(
            "No time entries available".to_string(),
        ));
    }

    // Build display items
    let items: Vec<String> = entries
        .iter()
        .map(|e| {
            let notes = e.notes.as_deref().unwrap_or("(no description)");

            let project_name = e
                .project
                .as_ref()
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown Project");

            let task_name = e
                .task
                .as_ref()
                .map(|t| t.name.as_str())
                .unwrap_or("Unknown Task");

            let hours_str = e
                .hours
                .map(|h| format!(" ({:.2}h)", h))
                .unwrap_or_default();

            let date_str = if e.spent_date != chrono::Local::now().format("%Y-%m-%d").to_string() {
                format!(" [{}]", e.spent_date)
            } else {
                String::new()
            };

            format!(
                "{} • {} > {}{}{}",
                notes, project_name, task_name, hours_str, date_str
            )
        })
        .collect();

    println!("\nSelect a time entry to continue:");

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Search and select entry")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|_| HarjiraError::UserCancelled)?;

    Ok(&entries[selection])
}
