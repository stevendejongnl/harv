# Install Systemd Timer

Set up the systemd timer to run harjira automatically on boot and hourly.

## Steps

1. Check if systemd files exist in `systemd/` directory
2. Verify the binary is installed at `~/.cargo/bin/harjira` (if not, run `cargo install --path .`)
3. Copy systemd files:
   - `cp systemd/harjira.service ~/.config/systemd/user/`
   - `cp systemd/harjira.timer ~/.config/systemd/user/`
4. Reload systemd: `systemctl --user daemon-reload`
5. Enable the timer: `systemctl --user enable harjira.timer`
6. Start the timer: `systemctl --user start harjira.timer`
7. Verify installation:
   - `systemctl --user status harjira.timer`
   - `systemctl --user list-timers | grep harjira`
8. Explain how to view logs: `journalctl --user -u harjira.service -f`
