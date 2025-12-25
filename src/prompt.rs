use crate::error::{HarjiraError, Result};
use crate::models::{HarvestProject, ProposedTimeEntry, Ticket, TimeEntry};
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Editor, Input, MultiSelect, Select};

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
            let new_hours: f64 = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Hours")
                .default(entry.hours)
                .validate_with(|input: &f64| -> std::result::Result<(), &str> {
                    if *input > 0.0 && *input <= 24.0 {
                        Ok(())
                    } else {
                        Err("Hours must be between 0 and 24")
                    }
                })
                .interact_text()
                .map_err(|_| HarjiraError::UserCancelled)?;

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
