# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`harv` is a Rust CLI tool for smart Harvest time tracking. It automatically creates Harvest time entries by scanning git commits for Jira ticket references (e.g., `PROJECT-123`), supports AI-powered time entry generation from natural language work summaries, and allows resuming work from previous time entries. The tool runs automatically via systemd timers or can be invoked manually for on-demand time tracking.

## Build & Test Commands

```bash
# Check compilation (fast, no build)
cargo check

# Run all tests
cargo test

# Run specific test
cargo test test_extract_basic_tickets

# Run tests in a specific module
cargo test ticket_parser::tests

# Build development binary
cargo build

# Build optimized release binary
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .

# Run with debug logging
RUST_LOG=debug cargo run -- sync --dry-run
```

## Architecture

### Core Flow (run_sync in main.rs:123)

The main sync operation follows this pipeline:

1. **Configuration Loading** → `Config::load()` reads `~/.config/harv/config.toml`
2. **Repository Discovery** → `git::discover_repositories()` finds git repos from config or current dir
3. **Commit Collection** → `git::get_commits_from_repositories()` gets today's commits from ALL local branches
4. **Ticket Extraction** → `ticket_parser::extract_tickets()` uses regex to find Jira tickets (case-insensitive)
5. **Jira Enrichment** → `JiraClient::get_issues()` fetches ticket summaries/status (fails gracefully per ticket)
6. **Ticket Selection** → Interactive prompt if multiple tickets, or auto-select based on settings
7. **Timer Conflict Resolution** → Checks `harvest_client.get_running_timer()`, prompts if timer exists
8. **Timer Creation** → `harvest_client.create_time_entry()` with format: `{TICKET-ID} - {Summary}`

### Module Responsibilities

- **main.rs**: CLI orchestration using `clap`. Default command is `sync`, which runs the full pipeline.
- **git.rs**: Walks ALL local branches (not just HEAD) using `git2`, filters commits by today's date range (00:00:00 to now). Uses `HashSet` to deduplicate commits across branches.
- **ticket_parser.rs**: Regex `(?i)\b([a-z]+)-(\d+)\b` extracts and normalizes tickets to uppercase. Returns sorted, deduplicated Vec.
- **jira.rs**: REST API client for `/rest/api/3/issue/{key}`. Handles 404/401/403 explicitly, falls back to placeholder tickets on errors.
- **harvest.rs**: REST API client for `/v2/time_entries`. Creates timers with `external_reference` linking to Jira. Supports dry-run mode via `Context`.
- **config.rs**: TOML config at `~/.config/harv/config.toml`. Environment variable overrides (e.g., `HARVEST_ACCESS_TOKEN`). Validation ensures tokens aren't placeholder strings.
- **prompt.rs**: Interactive UI using `dialoguer` for multi-ticket selection and timer conflict confirmation.

### Critical Design Decisions

**Git Commit Scope**: The tool intentionally scans ALL local branches, not just the current branch. This catches all work from today regardless of branch switching. Commits are deduplicated by OID using a HashSet.

**Error Handling Philosophy**: API errors are logged but non-fatal. If Jira ticket fetch fails, a placeholder ticket is created with error message in summary. If one repo fails in multi-repo setup, others continue processing.

**Timer Conflict Logic**: Three-way decision:
- Timer for same ticket → early return (already tracking)
- Timer for different ticket + auto_stop → stop and start new
- Timer for different ticket + no auto_stop → prompt user

**Context vs Config**: `Context` holds runtime flags (dry_run, quiet, verbose), while `Config` holds persistent settings from TOML. Context is passed to API clients to respect dry-run mode.

## Configuration

Configuration lives at `~/.config/harv/config.toml` with 600 permissions. Four sections:

```toml
[harvest]
access_token = "..." # Personal Access Token from Harvest
account_id = "..."   # Harvest account ID
project_id = 123     # Optional: default project for time entries
task_id = 456        # Optional: default task for time entries

[jira]
access_token = "..." # Personal Access Token (PAT)
base_url = "https://your-company.atlassian.net"

[git]
repositories = []    # Empty = use current dir, or list of absolute paths

[settings]
auto_start = false         # Skip prompts when starting timers
auto_stop = false          # Skip prompts when stopping existing timers
auto_select_single = true  # Auto-select if only one ticket found
continue_days = 1          # Default lookback period for continue command (optional)
```

Initialize with: `harv config init`

## Shell Completions

The tool supports shell autocompletion for commands, subcommands, and flags. Completions can be installed automatically or generated manually for bash, zsh, and fish.

### Installation

**Automatic (Recommended)**:
```bash
# Auto-detects your shell and installs to the standard location
harv completions install
```

This command:
- Detects your shell from the `$SHELL` environment variable
- Creates completion files in standard locations:
  - **Zsh**: `~/.zfunc/_harv` (requires adding `fpath=(~/.zfunc $fpath)` to `~/.zshrc`)
  - **Bash**: `~/.local/share/bash-completion/completions/harv`
  - **Fish**: `~/.config/fish/completions/harv.fish`
- Provides instructions for enabling completions

**Manual Generation**:
```bash
# Generate completion script for a specific shell
harv completions generate bash > /path/to/completions/harv
harv completions generate zsh > ~/.zfunc/_harv
harv completions generate fish > ~/.config/fish/completions/harv.fish
```

### Usage

Once installed and sourced, you can use TAB to autocomplete:
- **Commands**: `harv <TAB>` → shows `sync`, `status`, `stop`, `add`, `continue`, `generate`, `config`, `completions`
- **Flags**: `harv sync --<TAB>` → shows `--auto-start`, `--auto-stop`, `--repo`, `--dry-run`, etc.
- **Subcommands**: `harv config <TAB>` → shows `init`, `show`, `validate`
- **Shell types**: `harv completions generate <TAB>` → shows `bash`, `zsh`, `fish`, etc.

## Systemd Integration

The tool is designed to run unattended via systemd user timers:

- **harv.timer**: OnBootSec=2min, OnUnitActiveSec=1h (runs hourly)
- **harv.service**: Executes `harv sync --quiet --auto-start --auto-stop`

Logs go to systemd journal: `journalctl --user -u harv.service -f`

## API Behavior

**Harvest API** (`https://api.harvestapp.com/v2`):
- Authentication: Bearer token + `Harvest-Account-Id` header + `User-Agent`
- Timer creation requires project_id + task_id (optional, can be set in config)
- `external_reference` creates bidirectional link between Harvest and Jira
- Time entries use `spent_date` in YYYY-MM-DD format (always today)

**Jira API** (`{base_url}/rest/api/3`):
- Authentication: Bearer token (Personal Access Token)
- Only fetches issue summary and status, minimal API surface
- GET `/issue/{key}` endpoint

## Testing Strategy

Unit tests focus on:
- **ticket_parser**: Regex edge cases (case insensitivity, word boundaries, deduplication)
- **git**: Repository validation and discovery logic

Integration tests would require mocking HTTP responses (mockito is in dev-dependencies).

When adding tests for API clients:
```rust
// Example pattern for mocking Harvest API
#[cfg(test)]
mod tests {
    use mockito::{mock, server_url};

    #[test]
    fn test_get_time_entries() {
        let _m = mock("GET", "/v2/time_entries")
            .with_header("content-type", "application/json")
            .with_body(r#"{"time_entries": []}"#)
            .create();
        // Test implementation
    }
}
```

## Common Development Scenarios

**Adding a new CLI command**: Add to `Commands` enum in main.rs, implement handler function, route in match statement at line 93.

**Modifying Jira ticket regex**: Edit `JIRA_TICKET_RE` in ticket_parser.rs. Current pattern assumes format `{LETTERS}-{NUMBERS}`. Update tests in same file.

**Adding new config options**:
1. Add field to relevant struct in config.rs (HarvestConfig/JiraConfig/GitConfig/Settings)
2. Update `create_template()` with example
3. Add validation in `validate()` if needed
4. Update README.md config section

**Changing dry-run behavior**: Check for `ctx.dry_run` before any mutating operation. See harvest.rs:160 and harvest.rs:239 for examples.

## Security Considerations

- Config file MUST have 600 permissions (enforced in config.rs:160)
- Never log full tokens, mask in display (config.rs:246)
- `.gitignore` excludes `config.toml` and `*.toml.local`
- Environment variables override config for CI/testing scenarios

## Debugging

```bash
# Dry run with verbose logging
RUST_LOG=debug harv sync --dry-run

# Test without git commits (should show "No Jira tickets found")
harv sync --repo /tmp/empty-repo

# Test multi-ticket selection (requires multiple commits with different tickets)
harv sync -v

# Check systemd timer status
systemctl --user status harv.timer
journalctl --user -u harv.service -n 50
```

## AI-Powered Time Entry Generation

The `harv generate` command uses AI (OpenAI or Anthropic Claude) to generate Harvest time entries from a natural language summary of your workday.

### Core Flow (run_generate in main.rs:362)

1. **Configuration Check** → Validates `ai.enabled = true` in config
2. **Summary Input** → User provides work summary (via argument or interactive editor)
3. **Context Gathering** → Fetches available projects, tasks, and existing entries from Harvest
4. **AI Generation** → Sends context + summary to AI provider, receives structured JSON with proposed entries
5. **Approval Flow** → Displays entries in interactive table, user selects which to create
6. **Entry Creation** → Creates stopped time entries (not running timers) via `create_stopped_time_entry()`

### Architecture

**AI Provider System** (`src/ai/mod.rs`):
- `AiProvider` trait for extensibility (OpenAI, Anthropic, future providers)
- `AiContext` struct containing projects, tasks, existing entries, target hours
- `build_prompt()` creates detailed context-rich prompts for AI
- `parse_response()` extracts JSON from AI responses (handles markdown code blocks)
- `create_provider()` factory function based on config

**Provider Implementations**:
- **OpenAI** (`src/ai/providers/openai.rs`): Uses `gpt-4o` model, structured output via `response_format: {type: "json_object"}`
- **Anthropic** (`src/ai/providers/anthropic.rs`): Uses `claude-3-5-sonnet-20241022`, structured output via system prompt

**Harvest API Extensions** (`src/harvest.rs`):
- `get_projects()` - Fetches all active projects (lines 225-256)
- `get_project_tasks(project_id)` - Fetches tasks for a project (lines 258-299)
- `get_all_available_tasks()` - Fetches all tasks across all projects with non-fatal error handling (lines 301-322)
- `calculate_remaining_hours(target)` - Calculates hours remaining to reach target (lines 324-329)
- `create_stopped_time_entry()` - Creates completed entry with hours, not a running timer (lines 331-399)

**Approval UI** (`src/prompt.rs`):
- `prompt_work_summary()` - Interactive editor for multi-line work summary (lines 85-97)
- `review_and_approve_entries()` - Multi-select interface with confidence scores, total hours display (lines 99-187)

### Configuration

Add to `~/.config/harv/config.toml`:
```toml
[ai]
enabled = false
provider = "openai"  # or "anthropic"
api_key = ""
# model = "gpt-4o"  # optional override
target_hours = 8.0
```

Environment variable overrides:
- `AI_ENABLED` - Enable/disable AI generation
- `AI_PROVIDER` - Override provider (openai/anthropic)
- `AI_API_KEY` - Override API key
- `AI_MODEL` - Override model
- `AI_TARGET_HOURS` - Override target hours

### Command Usage

```bash
# Interactive mode (prompts for summary)
harv generate

# With inline summary
harv generate "Fixed bugs in auth module, reviewed PRs, team meeting"

# Override provider
harv generate --provider anthropic "Work summary here"

# Auto-approve (skip confirmation)
harv generate --auto-approve "Work summary"

# Custom target hours
harv generate --target-hours 6.5 "Work summary"

# Dry run
harv generate --dry-run "Work summary"
```

### Prompt Engineering

The AI receives:
- User's work summary
- All active projects with IDs and names
- All available tasks with IDs and names
- Existing time entries for today
- Target hours and already-logged hours
- Instructions to allocate remaining hours across activities
- Expected JSON output format

AI returns JSON with:
```json
{
  "time_entries": [
    {
      "description": "Implemented feature X",
      "project_id": 12345,
      "task_id": 67890,
      "hours": 3.5,
      "confidence": 0.9
    }
  ]
}
```

### Error Handling

- **AI API failures**: Rate limits, invalid keys → clear error messages
- **Invalid project/task IDs**: Validation against fetched lists before creation
- **Individual entry failures**: Non-fatal, continues with other entries, reports at end
- **Empty/zero hours**: Rejected at parse stage
- **Malformed JSON**: Handles both raw JSON and markdown code blocks

### Security Considerations

- API keys stored in config with 600 permissions
- Keys masked in display output
- User summaries sent to external AI APIs (documented in config template)
- No prompt injection risk due to structured JSON output
- Validates all fields before creating entries

## Timer Continuation Feature

The `harv continue` command allows resuming work on previously tracked tasks by creating a new running timer from an existing stopped time entry.

### Core Flow (run_continue in main.rs:733)

1. **Configuration Loading** → `Config::load()` reads `~/.config/harv/config.toml`
2. **Date Range Calculation** → Determines lookback period (default: 1 day = today only, configurable with `--days` flag)
3. **Entry Fetching** → `harvest_client.get_time_entries_range()` fetches entries for date range
4. **Filtering** → Filters to stopped entries only (is_running == false) with valid project/task
5. **Entry Selection** → `prompt_entry_selection()` shows fuzzy searchable list with format: `{notes} • {project} > {task} ({hours}h) [{date}]`
6. **Timer Conflict Resolution** → Same logic as run_sync: checks for running timer, prompts if exists
7. **Timer Creation** → `start_timer_from_entry()` creates new running timer with today's date
8. **Original Entry Preservation** → Original stopped entry remains unchanged (creates audit trail)

### Command Usage

```bash
# Interactive mode (prompts user to choose restart vs new entry)
harv continue

# Force restart existing entry (preserves original date)
harv continue --restart

# Force create new timer for today
harv continue --new-entry

# Look back more days
harv continue --days 7        # Last 7 days

# Auto-start without prompts
harv continue --auto-start    # Skips timer conflict confirmation

# Combined flags
harv continue --restart --days 7 --auto-start

# Dry run
harv continue --dry-run       # Preview without creating timer
harv continue --restart --dry-run  # Preview restart
```

### Continue Modes

The `harv continue` command supports two modes:

1. **Restart** - Restarts the existing stopped entry
   - Preserves original `spent_date` (if entry from Jan 5, stays on Jan 5)
   - Resets hours to 0, starts accumulating from restart time
   - Modifies same entry (no new entry created)
   - Uses Harvest API: `PATCH /v2/time_entries/{id}/restart`
   - Use case: Continue work on same logical task over multiple days

2. **New Entry** - Creates new running timer for today
   - Always uses today's date regardless of original entry date
   - Creates separate entry (original entry preserved)
   - Use case: Track work separately by day

### Mode Selection

Three ways to control mode (priority: flags > config > prompt):

1. **CLI Flags** (highest priority):
   - `--restart` - Force restart existing entry
   - `--new-entry` - Force create new timer

2. **Config Setting** (fallback if no flags):
   ```toml
   [settings]
   continue_mode = "restart"  # or "new" or "ask"
   ```

3. **Interactive Prompt** (default if no flag/config):
   - User is prompted to choose each time
   - Shows context-aware warning if entry is from past date
   - Displays date implications for each option

### Key Design Decisions

**Date Behavior with Restart**: When restarting an entry from a past date, the timer runs on the **original date**, not today. Example: Restart entry from Jan 5 → timer accumulates hours on Jan 5. Harvest reports will show time on original date.

**Date Behavior with New Entry**: New running timer always uses today's date regardless of original entry date. This ensures the timer is trackable in today's context even when continuing work from previous days.

**Entry Preservation (New Entry Mode)**: Original stopped entry is never modified or deleted. The continue command creates a NEW running timer with the same project/task/notes. This maintains a complete audit trail.

**Timer Conflict Logic**: Reuses the same conflict resolution as `run_sync`:
- Timer already running for same task (matching notes) → inform and skip
- Timer running for different task + auto_start → auto-stop and start new
- Timer running for different task + no auto_start → prompt user

**Default Lookback**: 1 day (today only) by default. This makes the selection list fast to scan and focuses on most recent work. Users can override with `--days` flag for longer lookback periods.

### Configuration

Add to `~/.config/harv/config.toml`:
```toml
[settings]
continue_days = 1  # Default lookback period (optional, defaults to 1 if not set)
continue_mode = "ask"  # How to continue: "restart", "new", or "ask" (default)
```

**Environment variable override**: `CONTINUE_MODE` - Set to "restart", "new", or "ask"

### API Methods

**Harvest API methods**:
- `get_time_entries_range(from_date, to_date)` (harvest.rs:107) - Fetches entries for date range via GET /v2/time_entries
- `restart_time_entry(entry_id, ctx)` (harvest.rs:267) - PATCH to `/v2/time_entries/{id}/restart`, preserves date and resets hours to 0
- `start_timer_from_entry(entry)` (harvest.rs:312) - Creates new running timer from existing entry for today, validates project/task exist

**Prompt functions**:
- `prompt_entry_selection(entries)` (prompt.rs:506) - Fuzzy searchable list of time entries
- `prompt_continue_mode(entry)` (prompt.rs:565) - Interactive selection between restart and new entry modes

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| No stopped entries in range | Display info message: "No stopped entries found today" (or "in last N days") |
| Entry missing project/task | Filtered out during selection (not shown to user) |
| Running timer for same task | Info message: "Timer already running for this task", early return |
| Running timer for different task | Prompt to stop (or auto-stop with --auto-start flag) |
| Restart entry from past date | Timer runs on original date (preserves spent_date) |
| New entry from past date | Creates timer for today, original entry preserved |
| Conflicting flags (--restart --new-entry) | Clap prevents this with `conflicts_with` attribute |
| Invalid continue_mode in config | Validation fails on config load with clear error message |
| Restart already-running entry | Harvest API returns error "Entry already running" |

### Testing Scenarios

Manual test cases:
1. `harv continue` (interactive) → Shows prompt with restart/new options
2. `harv continue --restart` → Directly restarts without prompt
3. `harv continue --new-entry` → Directly creates new timer without prompt
4. Restart entry from yesterday → Timer runs on yesterday's date (verify in Harvest)
5. New entry from yesterday → Timer created for today
6. Continue when timer already running for same task → Informs and skips
7. Continue when timer running for different task → Prompts to stop
8. Continue with --auto-start → Automatically stops conflicting timer
9. Continue with no stopped entries → Shows info message
10. `harv continue --restart --dry-run` → Previews restart without executing
11. `harv continue --new-entry --dry-run` → Previews new timer without executing
12. Config `continue_mode = "restart"` → Auto-restarts without prompt
13. Config `continue_mode = "new"` → Auto-creates new without prompt
14. Flag overrides config → CLI flag takes precedence over config setting

## Environment Variables

Override config for testing:
- `HARVEST_ACCESS_TOKEN`
- `HARVEST_ACCOUNT_ID`
- `JIRA_ACCESS_TOKEN`
- `JIRA_BASE_URL`
- `AI_ENABLED`
- `AI_PROVIDER`
- `AI_API_KEY`
- `AI_MODEL`
- `AI_TARGET_HOURS`
- `CONTINUE_MODE` - Set to "restart", "new", or "ask"
- `RUST_LOG` - Set to `debug` for verbose logging

## Known Limitations

- Only scans local branches (not remotes)
- Timer notes checked with `.contains()` for duplicate detection (not exact match)
- No support for Jira OAuth, only Personal Access Tokens
- Harvest project_id and task_id must be provided in config if not on time entry creation API call
- Date filtering uses local timezone, not UTC
