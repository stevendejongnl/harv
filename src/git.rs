use crate::error::{HarjiraError, Result};
use crate::models::Commit;
use chrono::{Local, TimeZone};
use git2::{BranchType, Repository};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::env;

/// Discover git repositories to check
///
/// If repositories are specified in config, use those.
/// Otherwise, use the current working directory.
pub fn discover_repositories(configured_repos: &[String]) -> Result<Vec<String>> {
    if configured_repos.is_empty() {
        // Use current working directory
        let cwd = env::current_dir()?;
        let cwd_str = cwd
            .to_str()
            .ok_or_else(|| HarjiraError::Config("Invalid current directory path".to_string()))?
            .to_string();

        // Verify it's a git repository
        if Repository::open(&cwd).is_ok() {
            Ok(vec![cwd_str])
        } else {
            Err(HarjiraError::ShowHelp)
        }
    } else {
        // Validate configured repositories
        let mut valid_repos = Vec::new();
        for repo_path in configured_repos {
            if Repository::open(repo_path).is_ok() {
                valid_repos.push(repo_path.clone());
            } else {
                warn!("Configured path is not a valid git repository: {}", repo_path);
            }
        }

        if valid_repos.is_empty() {
            return Err(HarjiraError::Config(
                "No valid git repositories found in configuration".to_string(),
            ));
        }

        Ok(valid_repos)
    }
}

/// Get all commits from today across all branches in a repository
pub fn get_todays_commits(repo_path: &str) -> Result<Vec<Commit>> {
    let repo = Repository::open(repo_path)?;

    // Calculate today's date range (00:00:00 to now)
    let today = Local::now().date_naive();
    let start_of_day = Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| HarjiraError::Git(git2::Error::from_str("Invalid datetime")))?
        .timestamp();
    let now = Local::now().timestamp();

    debug!(
        "Searching for commits between {} and {} in {}",
        start_of_day, now, repo_path
    );

    let mut all_commits = Vec::new();
    let mut seen_oids = HashSet::new();

    // Iterate through all local branches
    let branches = repo.branches(Some(BranchType::Local))?;

    for branch_result in branches {
        let (branch, _branch_type) = branch_result?;

        let branch_name = branch
            .name()?
            .unwrap_or("unknown")
            .to_string();

        debug!("Checking branch: {}", branch_name);

        // Get the commit that the branch points to
        if let Some(target) = branch.get().target() {
            let mut revwalk = repo.revwalk()?;
            revwalk.push(target)?;

            for oid_result in revwalk {
                let oid = oid_result?;

                // Skip if we've already processed this commit
                if seen_oids.contains(&oid) {
                    continue;
                }

                let commit = repo.find_commit(oid)?;
                let timestamp = commit.time().seconds();

                // Only include commits from today
                if timestamp >= start_of_day && timestamp <= now {
                    seen_oids.insert(oid);

                    let message = commit.message().unwrap_or("").to_string();
                    let author = commit
                        .author()
                        .name()
                        .unwrap_or("unknown")
                        .to_string();

                    debug!(
                        "Found commit from today: {} by {}",
                        &commit.message().unwrap_or("")[..50.min(commit.message().unwrap_or("").len())],
                        &author
                    );

                    all_commits.push(Commit {
                        message,
                        author,
                        timestamp,
                    });
                }

                // Stop walking if we've gone past today
                if timestamp < start_of_day {
                    break;
                }
            }
        }
    }

    // Sort by timestamp (most recent first)
    all_commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    info!(
        "Found {} commits from today in {}",
        all_commits.len(),
        repo_path
    );

    Ok(all_commits)
}

/// Get commits from today across multiple repositories
pub fn get_commits_from_repositories(repo_paths: &[String]) -> Result<Vec<Commit>> {
    let mut all_commits = Vec::new();

    for repo_path in repo_paths {
        match get_todays_commits(repo_path) {
            Ok(mut commits) => {
                all_commits.append(&mut commits);
            }
            Err(e) => {
                warn!("Failed to get commits from {}: {}", repo_path, e);
                // Continue with other repositories
            }
        }
    }

    // Sort all commits by timestamp (most recent first)
    all_commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(all_commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_repositories_validates_git() {
        // Current directory should be a git repo for this test to pass
        // or it should fail with appropriate error
        let result = discover_repositories(&[]);

        // Either we're in a git repo and get Ok, or we're not and get specific error
        match result {
            Ok(repos) => {
                assert_eq!(repos.len(), 1);
            }
            Err(HarjiraError::Config(msg)) => {
                assert!(msg.contains("not a git repository"));
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_discover_repositories_with_invalid_path() {
        let invalid_repos = vec!["/nonexistent/path".to_string()];
        let result = discover_repositories(&invalid_repos);

        assert!(result.is_err());
    }
}
