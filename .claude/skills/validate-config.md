# Validate Configuration

Check if the harjira configuration is set up correctly and help fix any issues.

## Steps

1. Check if config file exists: `~/.config/harjira/config.toml`
2. If not, guide user to run: `harjira config init` (or `cargo run -- config init`)
3. Validate configuration: `harjira config validate` (or `cargo run -- config validate`)
4. If validation fails, identify the issue:
   - Missing or invalid Harvest access token
   - Missing or invalid Jira access token
   - Invalid URLs or account IDs
   - Placeholder values not replaced
5. Show masked configuration: `harjira config show`
6. Verify file permissions: `ls -la ~/.config/harjira/config.toml` (should be 600)
7. Remind about environment variable overrides if needed
8. Test API connectivity if possible with dry-run
