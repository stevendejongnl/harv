# CLAUDE.md

Guidance for Claude Code working with `harv`, a Rust CLI for smart Harvest time tracking.

## Overview

`harv` scans git commits for Jira ticket references (e.g., `PROJECT-123`), creates Harvest time entries, generates entries from natural language summaries via AI, and resumes work from previous entries. Runs via systemd timers or manual invocation.

## Build & Test

```bash
cargo check              # Fast check
cargo test               # All tests
cargo test test_name     # Specific test
cargo build --release    # Optimized binary
RUST_LOG=debug cargo run -- sync --dry-run  # Debug logging
```

## Architecture

### Core Sync Flow (main.rs:123)

1. Load config (`~/.config/harv/config.toml`)
2. Discover repos (from config or current dir)
3. Collect today's commits from ALL local branches
4. Extract Jira tickets (regex: case-insensitive)
5. Fetch ticket summaries from Jira (fails gracefully per ticket)
6. Select ticket (interactive if multiple, auto-select if configured)
7. Check for running timer, prompt if conflict exists
8. Create entry: `{TICKET-ID} - {Summary}`

### Modules

| Module | Responsibility |
|--------|-----------------|
| **main.rs** | CLI via `clap`. Default: `sync` command |
| **git.rs** | Scans ALL local branches, deduplicates by OID, filters by today's date range |
| **ticket_parser.rs** | Regex `(?i)\b([a-z]+)-(\d+)\b` → sorted, deduplicated Vec |
| **jira.rs** | REST client `/rest/api/3/issue/{key}`, graceful error fallback |
| **harvest.rs** | REST client `/v2/time_entries`, respects `ctx.dry_run` |
| **config.rs** | TOML at `~/.config/harv/config.toml`, env var overrides |
| **prompt.rs** | Interactive UI via `dialoguer` |

### Design Decisions

- **Git Scope**: Scans ALL local branches (not just HEAD) to catch work across branch switches. Deduplicates via HashSet.
- **Error Handling**: API errors non-fatal. Jira fetch failure → placeholder ticket. Multi-repo: failures don't stop others.
- **Timer Conflict**: Same ticket → skip; different ticket + auto_stop → stop & start; different ticket + no auto_stop → prompt.
- **Context vs Config**: `Context` = runtime flags (dry_run, quiet, verbose). `Config` = persistent TOML settings.

## Configuration

`~/.config/harv/config.toml` (600 permissions):

```toml
[harvest]
access_token = "..."
account_id = "..."
project_id = 123       # Optional
task_id = 456          # Optional

[jira]
access_token = "..."
base_url = "https://your-company.atlassian.net"

[git]
repositories = []      # Empty = current dir

[settings]
auto_start = false
auto_stop = false
auto_select_single = true
continue_days = 1

[ai]
enabled = false
provider = "openai"    # or "anthropic"
api_key = ""
target_hours = 8.0
```

Initialize: `harv config init`

## Shell Completions

```bash
harv completions install              # Auto-detect shell, install
harv completions generate bash|zsh|fish  # Manual generation
```

Zsh: Add `fpath=(~/.zfunc $fpath)` to `~/.zshrc`

## API Behavior

**Harvest** (`https://api.harvestapp.com/v2`): Bearer token + `Harvest-Account-Id` header. `external_reference` links to Jira. `spent_date` always today.

**Jira** (`{base_url}/rest/api/3`): Bearer token (PAT). GET `/issue/{key}` for summary/status.

## Testing

- **ticket_parser**: Regex edge cases, deduplication
- **git**: Repository validation, discovery

Mocking template (mockito):
```rust
#[test]
fn test() {
    let _m = mock("GET", "/v2/time_entries")
        .with_header("content-type", "application/json")
        .with_body(r#"{"time_entries": []}"#)
        .create();
}
```

## Development Tasks

| Task | Steps |
|------|-------|
| **New CLI command** | Add to `Commands` enum (main.rs), implement handler, route in match (line 93) |
| **Modify Jira regex** | Edit `JIRA_TICKET_RE` in ticket_parser.rs, update tests |
| **Add config option** | Add field to struct (config.rs), update template, add validation |
| **Dry-run behavior** | Check `ctx.dry_run` before mutations (harvest.rs:160, 239) |

## Systemd Integration

- **harv.timer**: OnBootSec=2min, OnUnitActiveSec=1h
- **harv.service**: `harv sync --quiet --auto-start --auto-stop`
- Logs: `journalctl --user -u harv.service -f`

## Security

- Config: 600 permissions (enforced, config.rs:160)
- Tokens: Masked in display (config.rs:246)
- `.gitignore`: Excludes `config.toml`, `*.toml.local`
- Env vars override config for CI/testing

## Debugging

```bash
RUST_LOG=debug harv sync --dry-run    # Verbose logging
harv sync --repo /tmp/empty-repo      # Test empty repo
harv sync -v                          # Multi-ticket selection
systemctl --user status harv.timer    # Check timer
journalctl --user -u harv.service -n 50  # Last 50 logs
```

## AI-Powered Time Generation

`harv generate` creates entries from natural language summaries via OpenAI or Anthropic.

### Flow (main.rs:362)

1. Check `ai.enabled = true`
2. Get work summary (arg or editor)
3. Fetch projects, tasks, existing entries from Harvest
4. Send to AI, get JSON with proposed entries
5. Review & approve entries
6. Create stopped entries via `create_stopped_time_entry()`

### Architecture

**AI System** (`src/ai/mod.rs`):
- `AiProvider` trait (OpenAI, Anthropic extensible)
- `AiContext`: projects, tasks, entries, target hours
- `build_prompt()`: context-rich prompt
- `parse_response()`: JSON extraction (handles markdown blocks)

**Providers**:
- **OpenAI**: `gpt-4o` with `response_format: {type: "json_object"}`
- **Anthropic**: `claude-3-5-sonnet-20241022` with system prompt

**Harvest Extensions** (src/harvest.rs):
- `get_projects()`: All active projects (225-256)
- `get_project_tasks(id)`: Tasks for project (258-299)
- `get_all_available_tasks()`: All tasks, non-fatal errors (301-322)
- `calculate_remaining_hours(target)`: Remaining hours (324-329)
- `create_stopped_time_entry()`: Completed entry, not timer (331-399)

**UI** (src/prompt.rs):
- `prompt_work_summary()`: Multi-line editor (85-97)
- `review_and_approve_entries()`: Multi-select, confidence scores, total hours (99-187)

### Usage

```bash
harv generate                                         # Interactive
harv generate "Fixed bugs, reviewed PRs, meeting"    # Inline
harv generate --provider anthropic "Summary"         # Override provider
harv generate --auto-approve "Summary"               # Skip confirmation
harv generate --target-hours 6.5 "Summary"          # Custom target
harv generate --dry-run "Summary"                    # Preview
```

### AI Context & Response

**Sent to AI**:
- Work summary
- Active projects (IDs, names)
- Available tasks (IDs, names)
- Today's existing entries
- Target hours & already-logged hours
- Expected JSON format

**AI Returns**:
```json
{
  "time_entries": [
    {"description": "...", "project_id": 123, "task_id": 456, "hours": 3.5, "confidence": 0.9}
  ]
}
```

**Error Handling**: Rate limits, invalid keys → clear errors. Invalid IDs → validation before creation. Individual failures → non-fatal, report at end. Empty/zero hours → rejected. Malformed JSON → handled (raw or markdown).

## Timer Continuation

`harv continue` resumes work by creating new timer from existing stopped entry.

### Flow (main.rs:733)

1. Load config
2. Calculate date range (default: 1 day, `--days` flag to override)
3. Fetch stopped entries for range
4. Filter to stopped only, valid project/task
5. Fuzzy search selection: `{notes} • {project} > {task} ({hours}h) [{date}]`
6. Check for running timer, resolve conflict
7. Create new timer for today
8. Preserve original entry (audit trail)

### Modes

**Restart**: Preserves original `spent_date`, resets hours to 0, modifies same entry. Uses `PATCH /v2/time_entries/{id}/restart`. For continuing work over multiple days.

**New Entry**: Always today's date, creates separate entry, preserves original. For tracking by day.

### Selection (priority: flags > config > prompt)

```bash
harv continue                               # Interactive mode
harv continue --restart                     # Force restart
harv continue --new-entry                   # Force new timer
harv continue --days 7                      # Last 7 days
harv continue --auto-start                  # Auto-stop conflicts
harv continue --restart --days 7 --auto-start  # Combined
harv continue --dry-run                     # Preview
```

**Config**:
```toml
[settings]
continue_days = 1
continue_mode = "ask"  # or "restart"/"new"
```

**Env var**: `CONTINUE_MODE` → "restart", "new", "ask"

### Design Decisions

- **Restart Date**: Timer runs on original date (Harvest reports show original date)
- **New Entry Date**: Always today's date (ensures trackable in today's context)
- **Preservation**: Original entry never modified (audit trail)
- **Conflict**: Same task → inform & skip. Different task + auto_start → stop & start. Different task, no auto_start → prompt.
- **Default Lookback**: 1 day (fast scan, focuses on recent work)

### API Methods

- `get_time_entries_range(from, to)` (107): GET /v2/time_entries
- `restart_time_entry(id, ctx)` (267): PATCH /v2/time_entries/{id}/restart
- `start_timer_from_entry(entry)` (312): New timer for today, validates project/task
- `prompt_entry_selection(entries)` (506): Fuzzy search
- `prompt_continue_mode(entry)` (565): Restart vs new entry choice

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| No stopped entries | Info: "No stopped entries found" |
| Entry missing project/task | Filtered out, not shown |
| Timer running for same task | Info & skip |
| Timer for different task | Prompt (or auto-stop with flag) |
| Restart past-date entry | Timer on original date |
| New entry from past date | Timer created for today |
| Conflicting flags | Clap prevents via `conflicts_with` |
| Invalid continue_mode config | Validation fails on load |
| Restart already-running entry | Harvest API error |

## Environment Variables

Override config:
- `HARVEST_ACCESS_TOKEN`, `HARVEST_ACCOUNT_ID`
- `JIRA_ACCESS_TOKEN`, `JIRA_BASE_URL`
- `AI_ENABLED`, `AI_PROVIDER`, `AI_API_KEY`, `AI_MODEL`, `AI_TARGET_HOURS`
- `CONTINUE_MODE` → "restart", "new", "ask"
- `RUST_LOG` → "debug" for verbose

## Known Limitations

- Scans local branches only (not remotes)
- Timer duplicate detection via `.contains()` (not exact match)
- No Jira OAuth, only Personal Access Tokens
- project_id/task_id required in config or creation call
- Date filtering uses local timezone (not UTC)
