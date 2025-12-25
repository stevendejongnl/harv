# Run Tests

Run project tests with various options for different scenarios.

## Steps

1. Ask the user which tests to run (or default to all):
   - All tests: `cargo test`
   - Specific test: `cargo test <test_name>`
   - Specific module: `cargo test ticket_parser::tests`
   - With output: `cargo test -- --nocapture`
   - Verbose: `cargo test -- --test-threads=1 --nocapture`

2. Execute the chosen test command

3. Report results:
   - Number of tests passed/failed
   - Any test failures with details
   - Compilation warnings if any

4. For test failures, offer to:
   - Show the failing test code
   - Explain what the test is checking
   - Suggest potential fixes

5. Remind about test coverage areas:
   - ticket_parser: Jira ticket regex extraction
   - git: Repository discovery and validation
   - Integration tests in tests/ directory
