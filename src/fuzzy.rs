// ABOUTME: Fuzzy search implementation for matching user queries against SSH host entries
// ABOUTME: Optimized for sub-50ms search performance with support for case-insensitive matching

use crate::ssh::parser::HostEntry;

pub struct SearchEngine {
    hosts: Vec<HostEntry>,
}

impl SearchEngine {
    pub fn new(hosts: Vec<HostEntry>) -> Self {
        Self { hosts }
    }

    pub fn search(&self, query: &str, case_sensitive: bool, max_results: usize) -> Vec<&HostEntry> {
        if query.is_empty() {
            // Return all hosts up to max_results
            return self.hosts.iter().take(max_results).collect();
        }

        let query_lower = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut results: Vec<(&HostEntry, usize)> = self
            .hosts
            .iter()
            .filter_map(|host| {
                let score = calculate_fuzzy_score(&host.name, &query_lower, case_sensitive);
                if score > 0 { Some((host, score)) } else { None }
            })
            .collect();

        // Sort by score (higher is better)
        results.sort_by(|a, b| b.1.cmp(&a.1));

        // Return only the entries, limited by max_results
        results
            .into_iter()
            .take(max_results)
            .map(|(entry, _)| entry)
            .collect()
    }
}

fn calculate_fuzzy_score(target: &str, query: &str, case_sensitive: bool) -> usize {
    let target_normalized = if case_sensitive {
        target.to_string()
    } else {
        target.to_lowercase()
    };

    let query_normalized = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    // Exact match gets highest score
    if target_normalized == query_normalized {
        return 1000;
    }

    // Prefix match gets high score
    if target_normalized.starts_with(&query_normalized) {
        // Base score of 900, with bonus for shorter strings
        let length_bonus = 50 - target.len().min(50);
        let mut score = 900 + length_bonus;

        // Bonus if query is followed by a word boundary
        if target_normalized.len() > query_normalized.len() {
            let next_char = target_normalized
                .chars()
                .nth(query_normalized.len())
                .unwrap();
            if !next_char.is_alphanumeric() {
                score += 50; // Bonus for word boundary
            }
        }

        return score;
    }

    // Contains match gets medium score
    if target_normalized.contains(&query_normalized) {
        let position = target_normalized.find(&query_normalized).unwrap();
        // Base score of 700, minus position (earlier is better)
        return 700 - position.min(100); // Cap position penalty at 100
    }

    // Fuzzy match: all query characters appear in order
    let mut score = 0;
    let mut query_chars = query_normalized.chars();
    let mut current_query_char = query_chars.next();
    let mut consecutive_matches = 0;
    let mut match_positions = Vec::new();

    for (i, target_char) in target_normalized.chars().enumerate() {
        if let Some(qc) = current_query_char {
            if target_char == qc {
                match_positions.push(i);
                score += 100 + consecutive_matches * 10; // Bonus for consecutive matches
                consecutive_matches += 1;
                current_query_char = query_chars.next();
            } else {
                consecutive_matches = 0;
            }
        }
    }

    // Only return score if all query characters were found
    if current_query_char.is_none() && !match_positions.is_empty() {
        // Bonus for matches at the beginning
        if match_positions[0] == 0 {
            score += 50;
        }
        score
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn create_test_hosts(count: usize) -> Vec<HostEntry> {
        (0..count)
            .map(|i| {
                HostEntry::new(
                    format!("server{i}.example.com"),
                    format!("ssh server{i}.example.com"),
                )
            })
            .collect()
    }

    #[test]
    fn test_empty_query_returns_all() {
        let hosts = vec![
            HostEntry::new("server1".to_string(), "ssh server1".to_string()),
            HostEntry::new("server2".to_string(), "ssh server2".to_string()),
            HostEntry::new("server3".to_string(), "ssh server3".to_string()),
        ];

        let engine = SearchEngine::new(hosts);
        let results = engine.search("", false, 10);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_exact_match() {
        let hosts = vec![
            HostEntry::new("production".to_string(), "ssh production".to_string()),
            HostEntry::new("prod-backup".to_string(), "ssh prod-backup".to_string()),
            HostEntry::new("staging".to_string(), "ssh staging".to_string()),
        ];

        let engine = SearchEngine::new(hosts);
        let results = engine.search("production", false, 10);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "production");
    }

    #[test]
    fn test_prefix_match() {
        let hosts = vec![
            HostEntry::new("production".to_string(), "ssh production".to_string()),
            HostEntry::new("prod-backup".to_string(), "ssh prod-backup".to_string()),
            HostEntry::new("staging".to_string(), "ssh staging".to_string()),
        ];

        let engine = SearchEngine::new(hosts);
        let results = engine.search("prod", false, 10);

        assert_eq!(results.len(), 2);
        // Both are prefix matches, ordering may vary by length
        assert!(results.iter().any(|h| h.name == "production"));
        assert!(results.iter().any(|h| h.name == "prod-backup"));
    }

    #[test]
    fn test_fuzzy_match() {
        let hosts = vec![
            HostEntry::new(
                "development-server".to_string(),
                "ssh development-server".to_string(),
            ),
            HostEntry::new("test-server".to_string(), "ssh test-server".to_string()),
            HostEntry::new("devops".to_string(), "ssh devops".to_string()),
        ];

        let engine = SearchEngine::new(hosts);
        let results = engine.search("dev", false, 10);

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|h| h.name == "development-server"));
        assert!(results.iter().any(|h| h.name == "devops"));
    }

    #[test]
    fn test_case_sensitivity() {
        let hosts = vec![
            HostEntry::new("Server1".to_string(), "ssh Server1".to_string()),
            HostEntry::new("server2".to_string(), "ssh server2".to_string()),
        ];

        let engine = SearchEngine::new(hosts.clone());

        // Case insensitive
        let results = engine.search("server", false, 10);
        assert_eq!(results.len(), 2);

        // Case sensitive
        let results = engine.search("server", true, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "server2");
    }

    #[test]
    fn test_max_results_limit() {
        let hosts = create_test_hosts(100);
        let engine = SearchEngine::new(hosts);

        let results = engine.search("server", false, 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_performance_under_50ms() {
        let hosts = create_test_hosts(1000);
        let engine = SearchEngine::new(hosts);

        let start = Instant::now();
        let _results = engine.search("server42", false, 20);
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 50,
            "Search took {duration:?}, should be under 50ms"
        );
    }

    #[test]
    fn test_fuzzy_scoring_order() {
        let hosts = vec![
            HostEntry::new("github.com".to_string(), "ssh github.com".to_string()),
            HostEntry::new(
                "gitlab.company.com".to_string(),
                "ssh gitlab.company.com".to_string(),
            ),
            HostEntry::new("git.internal".to_string(), "ssh git.internal".to_string()),
            HostEntry::new("bitbucket.org".to_string(), "ssh bitbucket.org".to_string()),
        ];

        let engine = SearchEngine::new(hosts);
        let results = engine.search("git", false, 10);

        // Debug: Print scores
        // for host in &engine.hosts {
        //     let score = calculate_fuzzy_score(&host.name, "git", false);
        //     println!("{}: score={}", host.name, score);
        // }

        // All three hosts with "git" should be found
        assert_eq!(results.len(), 3);

        // git.internal should score highest (prefix match)
        assert_eq!(results[0].name, "git.internal");

        // github and gitlab should also be in results (contains match)
        assert!(results.iter().any(|h| h.name == "github.com"));
        assert!(results.iter().any(|h| h.name == "gitlab.company.com"));
    }
}
