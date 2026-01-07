# Validate Configuration

Check if the harv configuration is set up correctly and help fix any issues.

## Steps

1. Check if config file exists: `~/.config/harv/config.toml`
2. If not, guide user to run: `harv config init` (or `cargo run -- config init`)
3. Validate configuration: `harv config validate` (or `cargo run -- config validate`)
4. If validation fails, identify the issue:
   - Missing or invalid Harvest access token
   - Missing or invalid Jira access token
   - Invalid URLs or account IDs
   - Placeholder values not replaced
5. Show masked configuration: `harv config show`
6. Verify file permissions: `ls -la ~/.config/harv/config.toml` (should be 600)
7. Remind about environment variable overrides if needed
8. Test API connectivity if possible with dry-run
