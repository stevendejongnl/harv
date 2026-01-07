use crate::error::{HarjiraError, Result};
use chrono::{DateTime, Utc};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const USAGE_FILE_VERSION: u8 = 1;

/// Cache of project and task usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCache {
    version: u8,
    #[serde(default)]
    projects: HashMap<u64, UsageRecord>,
    #[serde(default)]
    tasks: HashMap<u64, UsageRecord>,
}

/// Record of when and how often an item was used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    last_used: DateTime<Utc>,
    use_count: u64,
}

/// Score for sorting items by usage
#[derive(Debug, Clone, Copy)]
pub struct UsageScore {
    pub last_used: DateTime<Utc>,
    pub use_count: u64,
}

impl UsageCache {
    /// Create a new empty usage cache
    pub fn new() -> Self {
        Self {
            version: USAGE_FILE_VERSION,
            projects: HashMap::new(),
            tasks: HashMap::new(),
        }
    }

    /// Load usage cache from disk, returns empty cache if file doesn't exist or is corrupt
    pub fn load() -> Result<Self> {
        match Self::load_internal() {
            Ok(cache) => {
                debug!(
                    "Loaded usage cache with {} projects, {} tasks",
                    cache.projects.len(),
                    cache.tasks.len()
                );
                Ok(cache)
            }
            Err(e) => {
                // Check if it's just a missing file (expected on first run)
                let path = usage_file_path()?;
                if !path.exists() {
                    debug!("No usage cache file found, starting fresh");
                } else {
                    warn!("Failed to load usage cache: {}. Starting fresh.", e);
                }
                Ok(Self::new())
            }
        }
    }

    /// Internal load function that can fail
    fn load_internal() -> Result<Self> {
        let path = usage_file_path()?;
        let contents = fs::read_to_string(&path)?;

        let cache: UsageCache = serde_json::from_str(&contents)?;

        // Validate version
        if cache.version > USAGE_FILE_VERSION {
            return Err(HarjiraError::Config(format!(
                "Usage cache version {} is newer than supported version {}",
                cache.version, USAGE_FILE_VERSION
            )));
        }

        Ok(cache)
    }

    /// Save usage cache to disk, logs errors but doesn't fail
    pub fn save(&self) -> Result<()> {
        if let Err(e) = self.save_internal() {
            warn!(
                "Failed to save usage cache: {}. Usage tracking will not persist.",
                e
            );
        }
        Ok(())
    }

    /// Internal save function that can fail
    fn save_internal(&self) -> Result<()> {
        let path = usage_file_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically using temp file + rename
        let temp_path = path.with_extension("tmp");
        let json = serde_json::to_string_pretty(self)?;

        fs::write(&temp_path, json)?;

        // Set permissions to 600 (user read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&temp_path, perms)?;
        }

        // Atomic rename
        fs::rename(&temp_path, &path)?;

        debug!("Saved usage cache to {}", path.display());
        Ok(())
    }

    /// Record that a project was used
    pub fn record_project_usage(&mut self, project_id: u64) {
        self.projects
            .entry(project_id)
            .and_modify(|record| {
                record.last_used = Utc::now();
                record.use_count += 1;
            })
            .or_insert(UsageRecord {
                last_used: Utc::now(),
                use_count: 1,
            });
        debug!("Recorded project usage: {}", project_id);
    }

    /// Record that a task was used
    pub fn record_task_usage(&mut self, task_id: u64) {
        self.tasks
            .entry(task_id)
            .and_modify(|record| {
                record.last_used = Utc::now();
                record.use_count += 1;
            })
            .or_insert(UsageRecord {
                last_used: Utc::now(),
                use_count: 1,
            });
        debug!("Recorded task usage: {}", task_id);
    }

    /// Get usage score for a project
    pub fn get_project_score(&self, project_id: u64) -> Option<UsageScore> {
        self.projects.get(&project_id).map(|record| UsageScore {
            last_used: record.last_used,
            use_count: record.use_count,
        })
    }

    /// Get usage score for a task
    pub fn get_task_score(&self, task_id: u64) -> Option<UsageScore> {
        self.tasks.get(&task_id).map(|record| UsageScore {
            last_used: record.last_used,
            use_count: record.use_count,
        })
    }
}

/// Sort items by usage, with most recently used first
/// Items with no usage data are sorted alphabetically at the end
pub fn sort_by_usage<T>(mut items: Vec<T>, score_fn: impl Fn(&T) -> Option<UsageScore>) -> Vec<T>
where
    T: HasName,
{
    items.sort_by(|a, b| {
        let score_a = score_fn(a);
        let score_b = score_fn(b);

        match (score_a, score_b) {
            // Both have usage data
            (Some(sa), Some(sb)) => {
                // Primary: sort by recency (most recent first)
                match sb.last_used.cmp(&sa.last_used) {
                    Ordering::Equal => {
                        // Secondary: tie-break by use count (higher first)
                        sb.use_count.cmp(&sa.use_count)
                    }
                    other => other,
                }
            }
            // Only A has usage - A comes first
            (Some(_), None) => Ordering::Less,
            // Only B has usage - B comes first
            (None, Some(_)) => Ordering::Greater,
            // Neither has usage - alphabetical by name
            (None, None) => a.name().cmp(b.name()),
        }
    });

    items
}

/// Trait for items that have a name for alphabetical sorting
pub trait HasName {
    fn name(&self) -> &str;
}

/// Get the path to the usage cache file
fn usage_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        HarjiraError::Config("Could not determine config directory".to_string())
    })?;
    Ok(config_dir.join("harv").join("usage.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache_is_empty() {
        let cache = UsageCache::new();
        assert_eq!(cache.version, USAGE_FILE_VERSION);
        assert_eq!(cache.projects.len(), 0);
        assert_eq!(cache.tasks.len(), 0);
    }

    #[test]
    fn test_record_project_usage() {
        let mut cache = UsageCache::new();
        cache.record_project_usage(123);

        assert_eq!(cache.projects.len(), 1);
        let score = cache.get_project_score(123).unwrap();
        assert_eq!(score.use_count, 1);
    }

    #[test]
    fn test_record_project_usage_increments() {
        let mut cache = UsageCache::new();
        cache.record_project_usage(123);
        cache.record_project_usage(123);

        let score = cache.get_project_score(123).unwrap();
        assert_eq!(score.use_count, 2);
    }

    #[test]
    fn test_record_task_usage() {
        let mut cache = UsageCache::new();
        cache.record_task_usage(456);

        assert_eq!(cache.tasks.len(), 1);
        let score = cache.get_task_score(456).unwrap();
        assert_eq!(score.use_count, 1);
    }

    #[test]
    fn test_get_score_for_missing_item() {
        let cache = UsageCache::new();
        assert!(cache.get_project_score(999).is_none());
        assert!(cache.get_task_score(999).is_none());
    }

    #[derive(Debug)]
    struct TestItem {
        id: u64,
        name: String,
    }

    impl HasName for TestItem {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_sort_by_usage_no_usage_data() {
        let items = vec![
            TestItem { id: 1, name: "Charlie".to_string() },
            TestItem { id: 2, name: "Alice".to_string() },
            TestItem { id: 3, name: "Bob".to_string() },
        ];

        let sorted = sort_by_usage(items, |_| None);
        assert_eq!(sorted[0].name, "Alice");
        assert_eq!(sorted[1].name, "Bob");
        assert_eq!(sorted[2].name, "Charlie");
    }

    #[test]
    fn test_sort_by_usage_with_usage_data() {
        let mut cache = UsageCache::new();

        // Record usage with different timestamps
        cache.record_project_usage(2); // Alice - first
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.record_project_usage(3); // Bob - second (more recent)

        let items = vec![
            TestItem { id: 1, name: "Charlie".to_string() }, // No usage
            TestItem { id: 2, name: "Alice".to_string() },   // Used first
            TestItem { id: 3, name: "Bob".to_string() },     // Used second (most recent)
        ];

        let sorted = sort_by_usage(items, |item| cache.get_project_score(item.id));

        // Bob should be first (most recent)
        assert_eq!(sorted[0].name, "Bob");
        // Alice should be second
        assert_eq!(sorted[1].name, "Alice");
        // Charlie should be last (no usage)
        assert_eq!(sorted[2].name, "Charlie");
    }

    #[test]
    fn test_sort_by_usage_tie_break_by_count() {
        let mut cache = UsageCache::new();

        // Record usage for both items at "same" time
        cache.record_project_usage(2); // Alice - count 1
        cache.record_project_usage(3); // Bob - count 1
        cache.record_project_usage(3); // Bob - count 2

        let items = vec![
            TestItem { id: 2, name: "Alice".to_string() },
            TestItem { id: 3, name: "Bob".to_string() },
        ];

        let sorted = sort_by_usage(items, |item| cache.get_project_score(item.id));

        // Bob should be first (higher use count)
        assert_eq!(sorted[0].name, "Bob");
        assert_eq!(sorted[1].name, "Alice");
    }
}
