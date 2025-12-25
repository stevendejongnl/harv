# Debug Logs

View and analyze systemd logs for the harjira timer service to troubleshoot issues.

## Steps

1. Check if the systemd timer is installed and running:
   - `systemctl --user status harjira.timer`
   - `systemctl --user list-timers | grep harjira`
2. Show recent logs: `journalctl --user -u harjira.service -n 50`
3. Show live logs: `journalctl --user -u harjira.service -f`
4. Check for common issues:
   - Configuration errors (missing tokens)
   - No git commits found
   - API authentication failures
   - Timer execution frequency
5. If timer isn't running, suggest troubleshooting steps
6. Show when the timer will run next
