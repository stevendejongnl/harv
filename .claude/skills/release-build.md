# Release Build

Build and install an optimized release version of harjira.

## Steps

1. Run tests to ensure everything works: `cargo test`
2. Build release binary: `cargo build --release`
3. Show the binary location: `target/release/harjira`
4. Install to `~/.cargo/bin`: `cargo install --path .`
5. Verify installation: `harjira --version`
6. If systemd timer is installed, suggest restarting it: `systemctl --user restart harjira.timer`
7. Confirm the installed version with: `which harjira` and test with `harjira status`
