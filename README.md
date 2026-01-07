# harv

**Smart Harvest time tracking with git commit integration and AI-powered time entry generation**

`harv` is a Rust CLI tool that makes Harvest time tracking effortless. It automatically creates time entries by detecting Jira tickets in your git commits, supports AI-powered time entry generation from natural language summaries, and can resume work from previous entries. It runs on system boot and hourly via systemd, ensuring your time tracking stays in sync with your actual work.

## Features

- Automatically detects Jira tickets from today's git commits across all branches
- Creates Harvest timers with Jira ticket details and links
- Prompts for user input when multiple tickets are found
- Handles timer conflicts (ask before stopping existing timers)
- Runs automatically on boot and every hour via systemd
- Supports multiple git repositories
- Case-insensitive Jira ticket matching (PROJECT-123, proj-456, etc.)
- Dry-run mode to preview changes
- Comprehensive status and configuration commands

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- Git
- Systemd (for automatic timer functionality)
- Harvest account with API access
- Jira account with API access

### Build from source

```bash
# Clone the repository
cd /home/stevendejong/workspace/cloudsuite/harvest-and-jira

# Build in release mode
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/harv`.

## Migrating from harjira

If you were previously using the `harjira` tool, migration is straightforward:

### Config Migration (Automatic)

Your configuration will be **automatically migrated** on the first run of any `harv` command. The tool will copy `~/.config/harjira/` to `~/.config/harv/` if the old directory exists and the new one doesn't.

```bash
# Simply run any harv command to trigger migration
harv config show
# Output: "Migrated config from ~/.config/harjira/ to ~/.config/harv/"
```

### Systemd Timer Migration (Run Script)

If you have the systemd timer installed, use the provided migration script:

```bash
# From the harjira project directory
./migrate-harjira-to-harv.sh
```

This script will:
1. Stop and disable the old `harjira` timer
2. Remove old systemd files
3. Install and start the new `harv` timer

Alternatively, migrate manually:

```bash
# Stop old timer
systemctl --user stop harjira.timer
systemctl --user disable harjira.timer
rm -f ~/.config/systemd/user/harjira.{service,timer}

# Install new timer
make systemd-install
```

### Binary Reinstallation

After updating the code, reinstall the binary:

```bash
cargo install --path .
# The new binary will be at ~/.cargo/bin/harv
```

## Configuration

### 1. Create API Credentials

**Harvest**:
1. Go to https://id.getharvest.com/developers
2. Create a new Personal Access Token
3. Note your Account ID and Access Token

**Jira**:
1. Go to https://id.atlassian.com/manage-profile/security/api-tokens
2. Create a new API token
3. Note your Jira base URL (e.g., `https://your-company.atlassian.net`)

### 2. Initialize Configuration

```bash
harv config init
```

This creates a configuration file at `~/.config/harv/config.toml` with secure permissions (600).

### 3. Edit Configuration

Edit `~/.config/harv/config.toml` and add your credentials:

```toml
[harvest]
access_token = "your_harvest_access_token_here"
account_id = "1234567"
user_agent = "harv (your.email@example.com)"

# Optional: Default project and task IDs
# Get these from: https://api.harvestapp.com/v2/projects
# project_id = 12345678
# task_id = 87654321

[jira]
access_token = "your_jira_personal_access_token_here"
base_url = "https://your-company.atlassian.net"

[git]
# Leave empty to use current working directory
# Or specify paths to git repositories to monitor
repositories = []

[settings]
# Skip prompts and automatically start timers
auto_start = false
# Skip prompts and automatically stop existing timers
auto_stop = false
# Automatically select ticket if only one is found
auto_select_single = true
```

### 4. Validate Configuration

```bash
harv config validate
```

## Usage

### Manual Sync

Check commits and sync to Harvest:

```bash
harv sync
```

Or simply:

```bash
harv
```

### Check Status

View current timer and today's entries:

```bash
harv status
```

Example output:
```
Harvest Timer Status
====================

✓ Timer Running
  Notes: PROJ-123 - Implement OAuth2 authentication
  Project: Backend Development
  Task: Programming
  Started: 14:30:00
  Duration: 2.50 hours

Today's Time Entries:
  • 1.50h - PROJ-122 - Code review
  • 2.50h - PROJ-123 - Implement OAuth2 authentication (running)

Total Time Today: 4.00 hours
```

### Stop Current Timer

```bash
harv stop
```

### Configuration Management

```bash
# Show current configuration (tokens masked)
harv config show

# Validate configuration
harv config validate
```

### Command Options

```bash
harv sync [OPTIONS]

Options:
  --auto-start           Automatically start timer without prompting
  --auto-stop            Automatically stop existing timer without prompting
  --repo <PATH>          Override repository path
  -n, --dry-run          Show what would happen without making changes
  -v, --verbose          Enable verbose logging
  -q, --quiet            Suppress non-essential output
```

## Systemd Integration

For automatic hourly checks and boot-time sync:

### Install Timer

```bash
# Copy systemd files
cp systemd/harv.service ~/.config/systemd/user/
cp systemd/harv.timer ~/.config/systemd/user/

# Reload systemd
systemctl --user daemon-reload

# Enable and start the timer
systemctl --user enable harv.timer
systemctl --user start harv.timer
```

### Verify Timer

```bash
# Check timer status
systemctl --user status harv.timer

# List all timers
systemctl --user list-timers

# View logs
journalctl --user -u harv.service -f
```

### Disable Timer

```bash
systemctl --user stop harv.timer
systemctl --user disable harv.timer
```

## How It Works

1. **Discovery**: Finds git repositories (from config or current directory)
2. **Commit Analysis**: Gets all commits from today across all local branches
3. **Ticket Extraction**: Parses Jira tickets using flexible pattern matching
4. **User Selection**: Prompts if multiple tickets found (unless auto mode)
5. **Jira Enrichment**: Fetches ticket summary and status from Jira API
6. **Harvest Integration**:
   - Checks for running timers
   - Handles conflicts (prompt or auto-stop)
   - Creates new timer with format: `{TICKET-ID} - {Summary}`
   - Links to Jira via external reference

## Jira Ticket Detection

The tool uses case-insensitive pattern matching to find Jira tickets in commit messages:

**Pattern**: `[A-Za-z]+-\d+`

**Examples**:
- `PROJECT-123` ✓
- `proj-456` ✓
- `Proj-789` ✓
- `CS-1` ✓
- `BACKEND-9999` ✓

**Commit Message Examples**:
```bash
git commit -m "CS-123: Fix authentication bug"
git commit -m "[PROJ-456] Add new feature"
git commit -m "Update docs for PROJECT-789"
```

## Examples

### Scenario 1: Single Ticket, No Running Timer

```bash
$ harv sync
Found 3 commits from today
Detected Jira ticket: PROJ-123
Fetching Jira details... PROJ-123 - Implement OAuth2 authentication
No timer currently running
✓ Started timer for PROJ-123 - Implement OAuth2 authentication
```

### Scenario 2: Multiple Tickets

```bash
$ harv sync
Found 5 commits from today

Multiple Jira tickets found in today's commits:
> PROJ-123 - Implement OAuth2 authentication [In Progress]
  PROJ-124 - Fix login redirect bug [To Do]
  CS-456 - Update documentation [Done]

Select a ticket to track:
> PROJ-123
✓ Started timer for PROJ-123 - Implement OAuth2 authentication
```

### Scenario 3: Timer Conflict

```bash
$ harv sync
Found 2 commits from today
Detected Jira ticket: PROJ-124

⚠️  Timer currently running:
   PROJ-123 - Implement OAuth2 authentication (Backend Development)
   Started at: 14:30:00
   Duration: 2.50 hours

New ticket: PROJ-124

Stop current timer and start new one? (y/N): y
✓ Stopped previous timer
✓ Started timer for PROJ-124 - Fix login redirect bug
```

### Scenario 4: Dry Run

```bash
$ harv sync --dry-run
Found 3 commits from today
Detected Jira ticket: PROJ-123
[DRY RUN] Would create time entry:
  Project ID: Some(12345678)
  Task ID: Some(87654321)
  Notes: PROJ-123 - Implement OAuth2 authentication
  External Reference: https://your-company.atlassian.net/browse/PROJ-123
```

## Environment Variables

Override configuration with environment variables (useful for CI/testing):

- `HARVEST_ACCESS_TOKEN` - Harvest API token
- `HARVEST_ACCOUNT_ID` - Harvest account ID
- `JIRA_ACCESS_TOKEN` - Jira API token
- `JIRA_BASE_URL` - Jira base URL

## Troubleshooting

### Configuration errors

```bash
# Validate your configuration
harv config validate

# View current configuration
harv config show
```

### No tickets found

- Ensure commits have Jira ticket references (e.g., `PROJECT-123`)
- Check that commits were made today
- Verify you're in a git repository or have repositories configured

### API errors

- Verify your Harvest and Jira tokens are valid
- Check account IDs and URLs in configuration
- Ensure you have necessary permissions

### Systemd timer not running

```bash
# Check timer status
systemctl --user status harv.timer

# View logs
journalctl --user -u harv.service -n 50

# Restart timer
systemctl --user restart harv.timer
```

## Development

### Run Tests

```bash
cargo test
```

### Run with Logging

```bash
RUST_LOG=debug harv sync --dry-run
```

### Build for Release

```bash
cargo build --release
```

## Project Structure

```
harv/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration management
│   ├── error.rs          # Error types
│   ├── models.rs         # Data structures
│   ├── git.rs            # Git operations
│   ├── harvest.rs        # Harvest API client
│   ├── jira.rs           # Jira API client
│   ├── ticket_parser.rs  # Jira ticket extraction
│   └── prompt.rs         # User interaction
├── systemd/
│   ├── harv.service   # Systemd service
│   └── harv.timer     # Systemd timer
└── tests/
    └── integration_tests.rs
```

## License

MIT

## Author

Steven de Jong

## Contributing

Contributions welcome! Please open an issue or pull request.

## Security

**Important**: Never commit your `config.toml` file with API credentials. The `.gitignore` file is configured to exclude it.

API tokens should be treated as passwords:
- Keep them secure
- Don't share them
- Rotate them regularly
- Use environment variables in CI/CD

## Links

- [Harvest API Documentation](https://help.getharvest.com/api-v2/)
- [Jira REST API Documentation](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)
- [Systemd Timer Documentation](https://www.freedesktop.org/software/systemd/man/systemd.timer.html)
