use clap::{CommandFactory, Parser, Subcommand};
use harv::*;
use log::{error, info};
use std::process;

#[derive(Parser)]
#[command(name = "harv")]
#[command(about = "Smart Harvest time tracking with git commit integration and AI-powered time entry generation", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Show what would happen without making changes
    #[arg(short = 'n', long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Check git commits and sync to Harvest (default command)
    Sync {
        /// Automatically start timer without prompting
        #[arg(long)]
        auto_start: bool,

        /// Automatically stop existing timer without prompting
        #[arg(long)]
        auto_stop: bool,

        /// Override repository path
        #[arg(long)]
        repo: Option<String>,
    },

    /// Show current Harvest timer status
    Status,

    /// Stop the currently running Harvest timer
    Stop,

    /// Manually add a time entry with interactive prompts
    Add,

    /// Continue work on an existing time entry by starting a new timer
    Continue {
        /// Number of days to look back for entries (default: 1)
        #[arg(long, short = 'd')]
        days: Option<u8>,

        /// Automatically start timer without prompting
        #[arg(long)]
        auto_start: bool,
    },

    /// Generate time entries from a work summary using AI
    Generate {
        /// Natural language summary of work done today
        /// If not provided, will prompt interactively
        summary: Option<String>,

        /// AI provider to use (overrides config)
        #[arg(long)]
        provider: Option<String>,

        /// Skip approval and create entries immediately
        #[arg(long)]
        auto_approve: bool,

        /// Target hours for the day (default: from config or 8.0)
        /// Supports decimal (e.g., 1.5) or colon format (e.g., 1:30)
        #[arg(long)]
        target_hours: Option<String>,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Create a template configuration file
    Init,

    /// Display current configuration
    Show,

    /// Validate configuration file
    Validate,
}

fn main() {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "error"
    } else {
        "info"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Build context
    let ctx = models::Context {
        dry_run: cli.dry_run,
        auto_start: false,
        auto_stop: false,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };

    // Run command
    let result = match cli.command {
        Some(Commands::Sync {
            auto_start,
            auto_stop,
            repo,
        }) => {
            let mut sync_ctx = ctx.clone();
            sync_ctx.auto_start = auto_start;
            sync_ctx.auto_stop = auto_stop;
            run_sync(sync_ctx, repo)
        }
        Some(Commands::Status) => run_status(ctx),
        Some(Commands::Stop) => run_stop(ctx),
        Some(Commands::Add) => run_add(ctx),
        Some(Commands::Continue { days, auto_start }) => {
            let mut continue_ctx = ctx.clone();
            continue_ctx.auto_start = auto_start;
            run_continue(continue_ctx, days)
        }
        Some(Commands::Generate {
            summary,
            provider,
            auto_approve,
            target_hours,
        }) => run_generate(ctx, summary, provider, auto_approve, target_hours),
        Some(Commands::Config { action }) => match action {
            ConfigAction::Init => run_config_init(),
            ConfigAction::Show => run_config_show(),
            ConfigAction::Validate => run_config_validate(),
        },
        None => {
            // Default to sync command
            run_sync(ctx, None)
        }
    };

    if let Err(e) = result {
        match e {
            HarjiraError::ShowHelp => {
                // Show help and exit cleanly
                Cli::command().print_help().ok();
                process::exit(0);
            }
            _ => {
                error!("{}", e);
                process::exit(1);
            }
        }
    }
}

fn run_sync(ctx: models::Context, repo_override: Option<String>) -> Result<()> {
    info!("Starting sync operation...");

    // Load configuration
    let config = Config::load()?;

    // Determine repositories to check
    let repos = if let Some(repo) = repo_override {
        vec![repo]
    } else {
        git::discover_repositories(&config.git.repositories)?
    };

    info!("Checking {} repository(ies)", repos.len());

    // Get commits from all repositories
    let commits = git::get_commits_from_repositories(&repos)?;

    if commits.is_empty() {
        if !ctx.quiet {
            prompt::display_info("No commits found from today");
        }
        return Ok(());
    }

    info!("Found {} commits from today", commits.len());

    // Extract commit messages
    let messages: Vec<String> = commits.iter().map(|c| c.message.clone()).collect();

    // Parse Jira tickets (with denylist filter)
    let ticket_keys = ticket_parser::extract_tickets(&messages, &config.ticket_filter.denylist);

    if ticket_keys.is_empty() {
        if !ctx.quiet {
            prompt::display_info("No Jira tickets found in today's commits");
        }
        return Ok(());
    }

    info!("Found {} Jira ticket(s): {:?}", ticket_keys.len(), ticket_keys);

    // Initialize API clients
    let jira_client = JiraClient::new(config.jira.clone())?;
    let harvest_client = HarvestClient::new(config.harvest.clone())?;

    // Fetch Jira details for all tickets
    let tickets = jira_client.get_issues(&ticket_keys);

    // Select ticket (prompt if multiple)
    let selected_ticket = if tickets.len() == 1 && config.settings.auto_select_single {
        tickets[0].clone()
    } else if tickets.len() == 1 || ctx.auto_start {
        tickets[0].clone()
    } else {
        prompt::prompt_ticket_selection(&tickets)?
    };

    info!("Selected ticket: {} - {}", selected_ticket.key, selected_ticket.summary);

    // Check current Harvest status
    let running_timer = harvest_client.get_running_timer()?;

    // Handle existing timer
    if let Some(timer) = running_timer {
        // Check if timer is already for this ticket
        if let Some(notes) = &timer.notes {
            if notes.contains(&selected_ticket.key) {
                if !ctx.quiet {
                    prompt::display_info(&format!(
                        "Timer already running for {}",
                        selected_ticket.key
                    ));
                }
                return Ok(());
            }
        }

        // Timer is for a different ticket
        let should_stop = if ctx.auto_stop {
            true
        } else {
            prompt::confirm_stop_timer(&timer, &selected_ticket.key)?
        };

        if !should_stop {
            if !ctx.quiet {
                prompt::display_info("Keeping current timer running");
            }
            return Ok(());
        }

        // Stop current timer
        harvest_client.stop_time_entry(timer.id, &ctx)?;
        if !ctx.quiet {
            prompt::display_success("Stopped previous timer");
        }
    }

    // Create new timer
    let jira_url = jira_client.get_ticket_url(&selected_ticket.key);
    harvest_client.create_time_entry(
        &selected_ticket.key,
        &selected_ticket.summary,
        &jira_url,
        &ctx,
    )?;

    if !ctx.quiet {
        prompt::display_success(&format!(
            "Started timer for {} - {}",
            selected_ticket.key, selected_ticket.summary
        ));
    }

    Ok(())
}

fn run_status(_ctx: models::Context) -> Result<()> {
    let config = Config::load()?;
    let harvest_client = HarvestClient::new(config.harvest)?;

    println!("\nHarvest Timer Status");
    println!("====================\n");

    let running_timer = harvest_client.get_running_timer()?;

    if let Some(timer) = running_timer {
        println!("✓ Timer Running");
        if let Some(notes) = &timer.notes {
            println!("  Notes: {}", notes);
        }
        if let Some(project) = &timer.project {
            println!("  Project: {}", project.name);
        }
        if let Some(task) = &timer.task {
            println!("  Task: {}", task.name);
        }
        if let Some(started) = &timer.started_time {
            println!("  Started: {}", started);
        }
        if let Some(hours) = timer.hours {
            println!("  Duration: {:.2} hours", hours);
        }
    } else {
        println!("⊗ No timer running");
    }

    println!();

    // Show today's entries
    let entries = harvest_client.get_todays_time_entries()?;
    if !entries.is_empty() {
        println!("Today's Time Entries:");
        for entry in &entries {
            let running_marker = if entry.is_running { " (running)" } else { "" };
            let hours = entry.hours.unwrap_or(0.0);
            let notes = entry.notes.as_deref().unwrap_or("No notes");
            println!("  • {:.2}h - {}{}", hours, notes, running_marker);
        }
    }

    // Calculate total
    let total_hours = harvest_client.get_total_hours_today()?;
    println!("\nTotal Time Today: {:.2} hours", total_hours);

    Ok(())
}

fn run_stop(ctx: models::Context) -> Result<()> {
    let config = Config::load()?;
    let harvest_client = HarvestClient::new(config.harvest)?;

    let running_timer = harvest_client.get_running_timer()?;

    if let Some(timer) = running_timer {
        harvest_client.stop_time_entry(timer.id, &ctx)?;
        if !ctx.quiet {
            prompt::display_success("Timer stopped");
        }
    } else {
        if !ctx.quiet {
            prompt::display_info("No timer currently running");
        }
    }

    Ok(())
}

fn run_config_init() -> Result<()> {
    Config::create_template()?;
    let config_path = Config::config_path()?;
    println!("✓ Configuration file created at: {}", config_path.display());
    println!("\nPlease edit the file and add your API credentials:");
    println!("  - Harvest access token: https://id.getharvest.com/developers");
    println!("  - Jira personal access token: https://id.atlassian.com/manage-profile/security/api-tokens");
    Ok(())
}

fn run_config_show() -> Result<()> {
    let config = Config::load()?;
    println!("\nCurrent Configuration");
    println!("====================\n");
    config.display();
    Ok(())
}

fn run_config_validate() -> Result<()> {
    let _config = Config::load()?;
    println!("✓ Configuration is valid");
    println!("  Config file: {}", Config::config_path()?.display());
    Ok(())
}

fn run_generate(
    ctx: models::Context,
    summary: Option<String>,
    provider_override: Option<String>,
    auto_approve: bool,
    target_hours_override: Option<String>,
) -> Result<()> {
    info!("Starting AI-powered time entry generation...");

    // Load configuration
    let mut config = Config::load()?;

    // Check if AI is enabled
    if !config.ai.enabled {
        return Err(HarjiraError::Config(
            "AI generation is not enabled. Set 'ai.enabled = true' in your config file."
                .to_string(),
        ));
    }

    // Apply overrides
    if let Some(provider) = provider_override {
        config.ai.provider = provider;
    }
    if let Some(target_str) = target_hours_override {
        let parsed = time_parser::parse_hours(&target_str)?;
        config.ai.target_hours = parsed;
    }

    // Get summary from user if not provided
    let work_summary = if let Some(s) = summary {
        s
    } else {
        prompt::prompt_work_summary()?
    };

    if work_summary.trim().is_empty() {
        return Err(HarjiraError::Config(
            "Work summary cannot be empty".to_string(),
        ));
    }

    // Initialize clients
    let harvest_client = HarvestClient::new(config.harvest.clone())?;
    let ai_provider = ai::create_provider(&config.ai)?;

    // Gather context for AI
    if !ctx.quiet {
        prompt::display_info("Fetching Harvest data...");
    }

    let projects = harvest_client.get_projects()?;
    let existing_entries = harvest_client.get_todays_time_entries()?;
    let today_total = harvest_client.get_total_hours_today()?;

    // Get all available tasks
    let all_tasks = harvest_client.get_all_available_tasks()?;
    let tasks: Vec<models::HarvestTask> = all_tasks.into_iter().map(|(_, task)| task).collect();

    let ai_context = ai::AiContext {
        available_projects: projects.clone(),
        available_tasks: tasks,
        existing_entries: existing_entries.clone(),
        target_hours: config.ai.target_hours,
        today_total_hours: today_total,
    };

    // Generate entries using AI
    if !ctx.quiet {
        prompt::display_info(&format!(
            "Generating time entries using {}...",
            ai_provider.name()
        ));
    }

    let mut proposed_entries = ai_provider.generate_time_entries(&work_summary, &ai_context)?;

    // Deduplicate entries based on description, project_id, task_id, and hours
    let mut seen = std::collections::HashSet::new();
    proposed_entries.retain(|entry| {
        let key = (
            entry.description.clone(),
            entry.project_id,
            entry.task_id,
            (entry.hours * 100.0) as i64, // Convert to cents to handle f64 comparison
        );
        seen.insert(key)
    });

    if proposed_entries.is_empty() {
        if !ctx.quiet {
            prompt::display_warning("AI did not generate any time entries");
        }
        return Ok(());
    }

    // Show proposed entries and get approval
    let approved_entries = if auto_approve || ctx.auto_start {
        proposed_entries
    } else {
        prompt::review_and_approve_entries(&proposed_entries, &projects)?
    };

    if approved_entries.is_empty() {
        if !ctx.quiet {
            prompt::display_info("No entries approved");
        }
        return Ok(());
    }

    // Get fallback project/task from most recent entry
    let fallback = existing_entries.first().and_then(|entry| {
        if let (Some(project), Some(task)) = (&entry.project, &entry.task) {
            Some((project.id, task.id))
        } else {
            None
        }
    });

    // Create time entries in Harvest
    let mut created_count = 0;
    let mut failed_count = 0;

    for entry in approved_entries {
        match harvest_client.create_stopped_time_entry(
            &entry.description,
            entry.project_id,
            entry.task_id,
            entry.hours,
            &ctx,
        ) {
            Ok(_) => {
                created_count += 1;
                if ctx.verbose {
                    prompt::display_success(&format!(
                        "Created: {} ({:.2}h)",
                        entry.description, entry.hours
                    ));
                }
            }
            Err(e) => {
                // Check if this is a 422 error (invalid project/task) and we have a fallback
                let is_422_error = e.to_string().contains("422 Unprocessable Entity");

                if is_422_error && fallback.is_some() {
                    let (fallback_project_id, fallback_task_id) = fallback.unwrap();

                    if !ctx.quiet {
                        prompt::display_warning(&format!(
                            "Invalid project/task for '{}'. Retrying with most recent project/task...",
                            entry.description
                        ));
                    }

                    // Retry with fallback project/task
                    match harvest_client.create_stopped_time_entry(
                        &entry.description,
                        fallback_project_id,
                        fallback_task_id,
                        entry.hours,
                        &ctx,
                    ) {
                        Ok(_) => {
                            created_count += 1;
                            if ctx.verbose {
                                prompt::display_success(&format!(
                                    "Created with fallback: {} ({:.2}h)",
                                    entry.description, entry.hours
                                ));
                            }
                        }
                        Err(retry_error) => {
                            failed_count += 1;
                            prompt::display_warning(&format!(
                                "Failed to create entry '{}' even with fallback: {}",
                                entry.description, retry_error
                            ));
                        }
                    }
                } else {
                    failed_count += 1;
                    prompt::display_warning(&format!(
                        "Failed to create entry '{}': {}",
                        entry.description, e
                    ));
                }
            }
        }
    }

    // Summary
    if !ctx.quiet {
        println!();
        if created_count > 0 {
            prompt::display_success(&format!(
                "Successfully created {} time entries",
                created_count
            ));
        }
        if failed_count > 0 {
            prompt::display_warning(&format!("{} entries failed", failed_count));
        }

        // Show new total
        let new_total = harvest_client.get_total_hours_today()?;
        println!("\nTotal time today: {:.2} hours", new_total);
    }

    Ok(())
}

fn run_add(ctx: models::Context) -> Result<()> {
    use crate::models::EntryType;

    info!("Starting manual time entry creation...");

    // Load configuration
    let config = Config::load()?;
    let harvest_client = HarvestClient::new(config.harvest.clone())?;

    // Load usage cache for sorting
    let mut usage_cache = usage::UsageCache::load()?;

    // Step 1: Select entry type
    let entry_type = prompt::prompt_entry_type()?;

    // Step 2: Select date
    let spent_date = prompt::prompt_date_selection()?;

    // Step 3: Fetch and select project
    if !ctx.quiet {
        prompt::display_info("Fetching available projects...");
    }
    let mut projects = harvest_client.get_projects()?;
    projects = usage::sort_by_usage(projects, |p| usage_cache.get_project_score(p.id));
    let selected_project = prompt::prompt_project_selection(&projects)?;

    // Step 4: Fetch and select task
    if !ctx.quiet {
        prompt::display_info("Fetching tasks...");
    }
    let mut tasks = harvest_client.get_project_tasks(selected_project.id)?;
    tasks = usage::sort_by_usage(tasks, |t| usage_cache.get_task_score(t.id));
    let selected_task = prompt::prompt_task_selection(&tasks)?;

    // Step 5: Enter description
    let description = prompt::prompt_description()?;

    // Step 6: Enter hours (only for stopped entries)
    let hours = if entry_type.is_running() {
        None
    } else {
        Some(prompt::prompt_hours()?)
    };

    // Step 7: Confirm
    let confirmed = prompt::confirm_entry_creation(
        &entry_type,
        &spent_date,
        &selected_project.name,
        &selected_task.name,
        &description,
        hours,
    )?;

    if !confirmed {
        if !ctx.quiet {
            prompt::display_info("Entry creation cancelled");
        }
        return Ok(());
    }

    // Step 8: Check for running timer (if creating running timer)
    if entry_type.is_running() {
        if let Some(timer) = harvest_client.get_running_timer()? {
            let should_stop = prompt::confirm_stop_timer_for_new(&timer)?;
            if !should_stop {
                if !ctx.quiet {
                    prompt::display_info("Keeping current timer running");
                }
                return Ok(());
            }
            harvest_client.stop_time_entry(timer.id, &ctx)?;
            if !ctx.quiet {
                prompt::display_success("Stopped previous timer");
            }
        }
    }

    // Step 9: Create entry
    match entry_type {
        EntryType::Running => {
            harvest_client.create_time_entry_with_date(
                &description,
                selected_project.id,
                selected_task.id,
                &spent_date,
                &ctx,
            )?;
            if !ctx.quiet {
                prompt::display_success(&format!(
                    "Started timer: {} - {}",
                    selected_project.name, description
                ));
            }
        }
        EntryType::Stopped => {
            let hours_val = hours.unwrap();
            harvest_client.create_stopped_time_entry_with_date(
                &description,
                selected_project.id,
                selected_task.id,
                hours_val,
                &spent_date,
                &ctx,
            )?;
            if !ctx.quiet {
                prompt::display_success(&format!(
                    "Created entry: {} ({:.2}h) on {}",
                    description, hours_val, spent_date
                ));
            }
        }
    }

    // Record usage for future sorting (skip in dry-run mode)
    if !ctx.dry_run {
        usage_cache.record_project_usage(selected_project.id);
        usage_cache.record_task_usage(selected_task.id);
        usage_cache.save()?;
    }

    // Show total for the date
    if !ctx.quiet {
        let total = harvest_client.get_total_hours_for_date(&spent_date)?;
        println!("\nTotal time on {}: {:.2} hours", spent_date, total);
    }

    Ok(())
}

fn run_continue(ctx: models::Context, days: Option<u8>) -> Result<()> {
    info!("Starting continue operation...");

    // Load configuration
    let config = Config::load()?;
    let harvest_client = HarvestClient::new(config.harvest.clone())?;

    // Determine lookback period (default: 1 day = today only)
    let lookback_days = days.unwrap_or(config.settings.continue_days.unwrap_or(1));

    // Calculate date range
    let today = chrono::Local::now();
    let from_date = if lookback_days == 1 {
        // Today only
        today.format("%Y-%m-%d").to_string()
    } else {
        // N days back
        let from = today - chrono::Duration::days((lookback_days - 1) as i64);
        from.format("%Y-%m-%d").to_string()
    };
    let to_date = today.format("%Y-%m-%d").to_string();

    // Fetch time entries for date range
    if !ctx.quiet {
        if lookback_days == 1 {
            prompt::display_info("Fetching today's time entries...");
        } else {
            prompt::display_info(&format!("Fetching entries from last {} days...", lookback_days));
        }
    }

    let all_entries = harvest_client.get_time_entries_range(&from_date, &to_date, &ctx)?;

    // Filter to stopped entries only (can't continue a running timer)
    let stopped_entries: Vec<_> = all_entries
        .into_iter()
        .filter(|e| !e.is_running)
        .collect();

    // Filter out entries without project/task (can't restart them)
    let valid_entries: Vec<_> = stopped_entries
        .into_iter()
        .filter(|e| e.project.is_some() && e.task.is_some())
        .collect();

    // Check if we have any entries to continue
    if valid_entries.is_empty() {
        let msg = if lookback_days == 1 {
            "No stopped time entries found today"
        } else {
            &format!("No stopped time entries found in last {} days", lookback_days)
        };
        if !ctx.quiet {
            prompt::display_info(msg);
        }
        return Ok(());
    }

    info!("Found {} valid entries to continue", valid_entries.len());

    // Prompt user to select entry
    let selected_entry = prompt::prompt_entry_selection(&valid_entries)?;

    let notes = selected_entry
        .notes
        .as_deref()
        .unwrap_or("(no description)");
    let project_name = selected_entry
        .project
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown");
    let task_name = selected_entry
        .task
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("Unknown");

    info!("Selected entry: {} - {}", project_name, notes);

    // Check for running timer conflicts
    let running_timer = harvest_client.get_running_timer()?;

    if let Some(timer) = running_timer {
        // Check if timer is already for this task (same notes)
        if let Some(timer_notes) = &timer.notes {
            if timer_notes == notes {
                if !ctx.quiet {
                    prompt::display_info(&format!(
                        "Timer already running for this task: {}",
                        notes
                    ));
                }
                return Ok(());
            }
        }

        // Timer is for a different task
        let should_stop = if ctx.auto_start {
            // auto_start implies auto_stop for continue command
            true
        } else {
            prompt::confirm_stop_timer(&timer, notes)?
        };

        if !should_stop {
            if !ctx.quiet {
                prompt::display_info("Keeping current timer running");
            }
            return Ok(());
        }

        // Stop current timer
        harvest_client.stop_time_entry(timer.id, &ctx)?;
        if !ctx.quiet {
            prompt::display_success("Stopped previous timer");
        }
    }

    // Start new timer from selected entry
    harvest_client.start_timer_from_entry(selected_entry, &ctx)?;

    if !ctx.quiet {
        prompt::display_success(&format!(
            "Started timer: {} > {} - {}",
            project_name, task_name, notes
        ));
    }

    Ok(())
}
