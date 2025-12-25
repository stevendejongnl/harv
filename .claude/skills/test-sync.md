# Test Sync Operation

Test the harjira sync operation with dry-run mode and verbose logging to preview what would happen without making actual changes.

## Steps

1. Run cargo check to ensure the project compiles
2. Build the project if needed
3. Run `cargo run -- sync --dry-run -v` to test sync with verbose output
4. If there's a running binary installed, also show how to test with: `harjira sync --dry-run -v`
5. Explain what the output means and what would happen in a real run
6. If errors occur, help diagnose them (missing config, no git commits, etc.)
