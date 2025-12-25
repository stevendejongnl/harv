use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

lazy_static! {
    /// Case-insensitive regex pattern for Jira tickets
    /// Matches patterns like: PROJECT-123, proj-456, Project-789
    static ref JIRA_TICKET_RE: Regex = Regex::new(r"(?i)\b([a-z]+)-(\d+)\b").unwrap();
}

/// Extract Jira ticket IDs from commit messages
///
/// Returns a deduplicated list of ticket IDs, normalized to uppercase
///
/// # Arguments
/// * `commit_messages` - List of commit messages to parse
/// * `denylist` - Optional list of ticket prefixes to filter out (case-insensitive)
pub fn extract_tickets(commit_messages: &[String], denylist: &[String]) -> Vec<String> {
    let mut tickets = HashSet::new();

    // Normalize denylist to uppercase for case-insensitive comparison
    let denylist_upper: Vec<String> = denylist.iter().map(|s| s.to_uppercase()).collect();

    for message in commit_messages {
        for cap in JIRA_TICKET_RE.captures_iter(message) {
            // Normalize to uppercase: PROJECT-123
            let prefix = cap[1].to_uppercase();
            let ticket = format!("{}-{}", prefix, &cap[2]);

            // Skip if ticket prefix is in denylist
            if denylist_upper.contains(&prefix) {
                continue;
            }

            tickets.insert(ticket);
        }
    }

    let mut result: Vec<String> = tickets.into_iter().collect();
    result.sort(); // Sort for consistent ordering
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_tickets() {
        let messages = vec![
            "CS-123: Fix authentication bug".to_string(),
            "PROJ-456: Add new feature".to_string(),
            "Update documentation for PROJECT-789".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 3);
        assert!(tickets.contains(&"CS-123".to_string()));
        assert!(tickets.contains(&"PROJ-456".to_string()));
        assert!(tickets.contains(&"PROJECT-789".to_string()));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let messages = vec![
            "cs-123: lowercase ticket".to_string(),
            "CS-123: uppercase ticket".to_string(),
            "Cs-123: mixed case ticket".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        // Should be deduplicated to one ticket
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0], "CS-123");
    }

    #[test]
    fn test_multiple_tickets_in_one_message() {
        let messages = vec!["Fix CS-123 and PROJ-456 together".to_string()];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 2);
        assert!(tickets.contains(&"CS-123".to_string()));
        assert!(tickets.contains(&"PROJ-456".to_string()));
    }

    #[test]
    fn test_no_tickets() {
        let messages = vec![
            "Regular commit message without tickets".to_string(),
            "Another commit, still no tickets".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 0);
    }

    #[test]
    fn test_ticket_at_various_positions() {
        let messages = vec![
            "PROJ-123 at the start".to_string(),
            "In the middle PROJ-456 of text".to_string(),
            "At the end PROJ-789".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 3);
    }

    #[test]
    fn test_deduplication_across_messages() {
        let messages = vec![
            "CS-123: First commit".to_string(),
            "CS-123: Second commit".to_string(),
            "CS-123: Third commit".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0], "CS-123");
    }

    #[test]
    fn test_word_boundaries() {
        // Should not match partial words
        let messages = vec![
            "test-123 is valid".to_string(),
            "notaproject-456 should not match".to_string(), // This will actually match
            "ABC-123XYZ should have boundaries".to_string(), // Won't match due to boundaries
        ];

        let tickets = extract_tickets(&messages, &[]);
        // Only TEST-123 and NOTAPROJECT-456 will match due to \b boundaries
        assert!(tickets.contains(&"TEST-123".to_string()));
        assert!(tickets.contains(&"NOTAPROJECT-456".to_string()));
        // ABC-123XYZ won't match because of word boundaries
        assert!(!tickets.iter().any(|t| t.contains("123XYZ")));
    }

    #[test]
    fn test_various_formats() {
        let messages = vec![
            "[CS-123] Commit with brackets".to_string(),
            "(PROJ-456) Commit with parentheses".to_string(),
            "Fixes: TEST-789".to_string(),
            "See also: ABC-111, DEF-222".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 5);
        assert!(tickets.contains(&"CS-123".to_string()));
        assert!(tickets.contains(&"PROJ-456".to_string()));
        assert!(tickets.contains(&"TEST-789".to_string()));
        assert!(tickets.contains(&"ABC-111".to_string()));
        assert!(tickets.contains(&"DEF-222".to_string()));
    }

    #[test]
    fn test_single_letter_projects() {
        let messages = vec!["A-123 single letter project".to_string()];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0], "A-123");
    }

    #[test]
    fn test_denylist_filters_tickets() {
        let messages = vec![
            "CWE-22: Security vulnerability fix".to_string(),
            "CVE-2024-1234: Security patch".to_string(),
            "PROJ-123: Real Jira ticket".to_string(),
        ];

        let denylist = vec!["CWE".to_string(), "CVE".to_string()];
        let tickets = extract_tickets(&messages, &denylist);

        assert_eq!(tickets.len(), 1);
        assert!(tickets.contains(&"PROJ-123".to_string()));
        assert!(!tickets.contains(&"CWE-22".to_string()));
        assert!(!tickets.contains(&"CVE-2024-1234".to_string()));
    }

    #[test]
    fn test_denylist_case_insensitive() {
        let messages = vec![
            "cwe-22: lowercase".to_string(),
            "CWE-123: uppercase".to_string(),
            "Cwe-456: mixed case".to_string(),
            "PROJ-789: valid ticket".to_string(),
        ];

        let denylist = vec!["CWE".to_string()];
        let tickets = extract_tickets(&messages, &denylist);

        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0], "PROJ-789");
    }

    #[test]
    fn test_empty_denylist() {
        let messages = vec![
            "CWE-22: Should be included".to_string(),
            "PROJ-123: Also included".to_string(),
        ];

        let tickets = extract_tickets(&messages, &[]);
        assert_eq!(tickets.len(), 2);
        assert!(tickets.contains(&"CWE-22".to_string()));
        assert!(tickets.contains(&"PROJ-123".to_string()));
    }
}
