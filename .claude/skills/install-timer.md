# Install Systemd Timer

Set up the systemd timer to run harv automatically on boot and hourly.

## Steps

1. Check if systemd files exist in `systemd/` directory
2. Verify the binary is installed at `~/.cargo/bin/harv` (if not, run `cargo install --path .`)
3. Copy systemd files:
   - `cp systemd/harv.service ~/.config/systemd/user/`
   - `cp systemd/harv.timer ~/.config/systemd/user/`
4. Reload systemd: `systemctl --user daemon-reload`
5. Enable the timer: `systemctl --user enable harv.timer`
6. Start the timer: `systemctl --user start harv.timer`
7. Verify installation:
   - `systemctl --user status harv.timer`
   - `systemctl --user list-timers | grep harv`
8. Explain how to view logs: `journalctl --user -u harv.service -f`
