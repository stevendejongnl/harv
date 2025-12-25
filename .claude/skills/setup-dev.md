# Setup Development Environment

Set up a complete development environment for harjira from scratch.

## Steps

1. Verify prerequisites:
   - Rust toolchain: `rustc --version` (should be 1.70+)
   - Cargo: `cargo --version`
   - Git: `git --version`
   - Systemd (for timer features)

2. Build the project:
   - `cargo check` - Fast compilation check
   - `cargo build` - Build debug binary
   - `cargo test` - Run all tests

3. Set up configuration:
   - `cargo run -- config init` to create template
   - Explain what credentials are needed:
     - Harvest: https://id.getharvest.com/developers
     - Jira: https://id.atlassian.com/manage-profile/security/api-tokens
   - Show config file location: `~/.config/harjira/config.toml`
   - Remind to set file permissions: `chmod 600 ~/.config/harjira/config.toml`

4. Test the setup:
   - `cargo run -- config validate`
   - `cargo run -- sync --dry-run -v`

5. Optional: Install systemd timer for automatic tracking
   - Refer to the install-timer skill

6. Show helpful development commands:
   - `cargo run -- status` - Check Harvest timer
   - `RUST_LOG=debug cargo run -- sync --dry-run` - Debug mode
   - `cargo test -- --nocapture` - Run tests with output

7. Remind about CLAUDE.md and README.md for architecture details
