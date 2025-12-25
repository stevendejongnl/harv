# Harjira Skills

This directory contains skills for working with the harjira project. Skills are reusable workflows that can be invoked in Claude Code.

## Available Skills

- **setup-dev** - Set up a complete development environment from scratch
- **quick-check** - Run fast validation (cargo check + tests)
- **run-tests** - Run project tests with various options
- **test-sync** - Test the sync operation with dry-run and verbose logging
- **validate-config** - Check if configuration is set up correctly
- **check-commits** - Analyze today's git commits for Jira tickets
- **release-build** - Build and install optimized release version
- **install-timer** - Set up systemd timer for automatic tracking
- **debug-logs** - View and analyze systemd logs

## Usage

In Claude Code, you can invoke skills using the Skill tool or by mentioning them in conversation. For example:

- "Run the quick-check skill"
- "Use the test-sync skill to see what would happen"
- "Help me with setup-dev"

## Workflow Examples

### First Time Setup
1. `setup-dev` - Set up development environment
2. `validate-config` - Ensure configuration is correct
3. `test-sync` - Test without making changes

### Development Cycle
1. Make code changes
2. `quick-check` - Verify compilation and tests
3. `run-tests` - Run specific tests if needed
4. `release-build` - Build and install when ready

### Troubleshooting
1. `check-commits` - See what tickets would be detected
2. `debug-logs` - View systemd logs if timer isn't working
3. `validate-config` - Check configuration issues
