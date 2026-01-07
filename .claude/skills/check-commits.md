# Check Commits

Analyze today's git commits to see what Jira tickets harv would detect.

## Steps

1. Check if we're in a git repository
2. Show today's commits across all local branches:
   ```bash
   git log --all --since="today 00:00" --pretty=format:"%h - %s (%an, %ar)"
   ```
3. Extract and highlight any Jira ticket patterns (case-insensitive):
   - Pattern: `[A-Za-z]+-\d+`
   - Examples: PROJECT-123, proj-456, CS-789
4. Show which tickets would be detected by harv
5. If no commits today, explain that harv would find nothing
6. If no Jira tickets in commits, suggest commit message format:
   - `git commit -m "PROJ-123: Description of change"`
   - `git commit -m "[PROJ-123] Description of change"`
7. Optionally show what a dry-run would do: `harv sync --dry-run`
