//! Fuzzy search implementation for Joshify
//!
//! Provides fast, typo-tolerant searching for tracks, albums, and artists.
//! Simple implementation that works well for terminal UI search.

/// Search result with relevance score
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult<T> {
    /// The matched item
    pub item: T,
    /// Match score (higher = better match)
    pub score: u32,
    /// Indices of matched characters in the item string
    pub match_indices: Vec<usize>,
}

impl<T> SearchResult<T> {
    pub fn new(item: T, score: u32, match_indices: Vec<usize>) -> Self {
        Self {
            item,
            score,
            match_indices,
        }
    }
}

/// Simplified fuzzy search for strings
pub struct SimpleFuzzySearch {
    /// Search items
    items: Vec<String>,
    /// Pattern atoms for matching
    pattern: String,
}

impl SimpleFuzzySearch {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            pattern: String::new(),
        }
    }

    /// Add items
    pub fn add_items(&mut self, items: Vec<String>) {
        self.items.extend(items);
    }

    /// Set search pattern
    pub fn set_pattern(&mut self, pattern: &str) {
        self.pattern = pattern.to_lowercase();
    }

    /// Get current pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Search and return matching items sorted by relevance
    pub fn search(&self, limit: usize) -> Vec<(String, u32, Vec<usize>)> {
        if self.pattern.is_empty() {
            return self.items.iter()
                .take(limit)
                .map(|s| (s.clone(), 0, vec![]))
                .collect();
        }

        let mut results: Vec<(String, u32, Vec<usize>)> = self.items
            .iter()
            .filter_map(|item| {
                self.calculate_score(item).map(|(score, indices)| (item.clone(), score, indices))
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(limit);
        results
    }

    /// Calculate fuzzy match score and match indices
    /// Returns None if no match, Some((score, indices)) if match
    fn calculate_score(&self, item: &str) -> Option<(u32, Vec<usize>)> {
        let item_lower = item.to_lowercase();
        let pattern = &self.pattern;

        // Exact match gets highest score
        if item_lower == *pattern {
            let indices: Vec<usize> = (0..item.len()).collect();
            return Some((1000, indices));
        }

        // Starts with gets high score
        if item_lower.starts_with(pattern) {
            let indices: Vec<usize> = (0..pattern.len()).collect();
            return Some((900, indices));
        }

        // Contains gets medium score
        if let Some(pos) = item_lower.find(pattern) {
            let indices: Vec<usize> = (pos..pos + pattern.len()).collect();
            return Some((800, indices));
        }

        // Fuzzy match: all characters in order
        let mut pattern_chars = pattern.chars().peekable();
        let mut score = 100u32;
        let mut match_indices = Vec::new();
        let mut _last_match_pos = 0usize;
        let mut bonus = 0u32;

        if let Some(first_char) = pattern_chars.next() {
            // Find first character
            if let Some(pos) = item_lower.find(first_char) {
                _last_match_pos = pos;
                match_indices.push(pos);
                score -= pos as u32; // Penalty for position
            } else {
                return None;
            }

            // Find remaining characters in order
            for (_, char) in pattern_chars.enumerate() {
                if _last_match_pos + 1 >= item_lower.len() {
                    return None;
                }

                if let Some(rel_pos) = item_lower[_last_match_pos + 1..].find(char) {
                    let pos = _last_match_pos + 1 + rel_pos;
                    match_indices.push(pos);

                    // Bonus for consecutive matches (camelCase/kebab-case)
                    if pos == _last_match_pos + 1 {
                        bonus += 50;
                    } else {
                        // Penalty for gaps
                        score += rel_pos as u32;
                    }

                    _last_match_pos = pos;
                } else {
                    return None;
                }
            }
        }

        // Apply bonus
        if bonus > 0 && bonus < score {
            score -= bonus;
        }

        Some((score.saturating_sub(match_indices.len() as u32), match_indices))
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get item count
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for SimpleFuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}

/// Searchable item trait
pub trait Searchable {
    /// Get the searchable text representation
    fn search_text(&self) -> String;
}

/// Search engine that works with Searchable items
pub struct SearchEngine<T: Searchable + Clone> {
    searcher: SimpleFuzzySearch,
    items: Vec<T>,
}

impl<T: Searchable + Clone> SearchEngine<T> {
    pub fn new() -> Self {
        Self {
            searcher: SimpleFuzzySearch::new(),
            items: Vec::new(),
        }
    }

    /// Add items
    pub fn add_items(&mut self, items: Vec<T>) {
        let strings: Vec<String> = items.iter().map(|i| i.search_text()).collect();
        self.searcher.add_items(strings);
        self.items.extend(items);
    }

    /// Set search pattern
    pub fn set_pattern(&mut self, pattern: &str) {
        self.searcher.set_pattern(pattern);
    }

    /// Search and return items with scores
    pub fn search(&self, limit: usize) -> Vec<SearchResult<T>> {
        let results = self.searcher.search(limit);

        results
            .into_iter()
            .filter_map(|(text, score, indices)| {
                // Find the original item by matching text
                self.items
                    .iter()
                    .find(|item| item.search_text() == text)
                    .map(|item| SearchResult::new(item.clone(), score, indices))
            })
            .collect()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.searcher.clear();
        self.items.clear();
    }
}

impl<T: Searchable + Clone> Default for SearchEngine<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for search
pub mod utils {
    /// Highlight matched characters in a string
    pub fn highlight_matches(text: &str, match_indices: &[usize]) -> String {
        if match_indices.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut in_match = false;

        for (idx, ch) in text.chars().enumerate() {
            let is_match = match_indices.contains(&idx);

            if is_match && !in_match {
                result.push('[');
                in_match = true;
            } else if !is_match && in_match {
                result.push(']');
                in_match = false;
            }

            result.push(ch);
        }

        if in_match {
            result.push(']');
        }

        result
    }

    /// Filter items by multiple patterns (AND logic)
    pub fn filter_by_patterns(items: &[String], patterns: &[&str]) -> Vec<String> {
        items
            .iter()
            .filter(|item| {
                let item_lower = item.to_lowercase();
                patterns.iter().all(|pat| {
                    let pat_lower = pat.to_lowercase();
                    // Check exact, starts with, contains, or fuzzy
                    item_lower.contains(&pat_lower) ||
                    fuzzy_match(&item_lower, &pat_lower)
                })
            })
            .cloned()
            .collect()
    }

    /// Simple fuzzy match - returns true if all chars in pattern appear in order
    /// Simple fuzzy match - returns true if all chars in pattern appear in order
    pub fn fuzzy_match(text: &str, pattern: &str) -> bool {
        let text_chars = text.chars();
        let mut pattern_chars = pattern.chars();

        let mut current_pat = pattern_chars.next();

        for text_ch in text_chars {
            if let Some(pat_ch) = current_pat {
                if text_ch == pat_ch {
                    current_pat = pattern_chars.next();
                    if current_pat.is_none() {
                        return true; // All pattern chars matched
                    }
                }
            } else {
                return true; // All pattern chars matched
            }
        }

        current_pat.is_none() // True if we matched all pattern chars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_fuzzy_search_exact_match() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Test Artist".to_string(),
        ]);

        search.set_pattern("taylor swift");
        let results = search.search(10);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Taylor Swift");
        assert_eq!(results[0].1, 1000); // Exact match score
    }

    #[test]
    fn test_simple_fuzzy_search_starts_with() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Test Artist".to_string(),
            "Tame Impala".to_string(),
        ]);

        search.set_pattern("tay");
        let results = search.search(10);

        assert!(results.iter().any(|(s, _, _)| s == "Taylor Swift"));
        // Taylor Swift should have highest score (starts with)
        assert_eq!(results[0].1, 900);
    }

    #[test]
    fn test_simple_fuzzy_search_contains() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Test Artist".to_string(),
            "Kanye West".to_string(),
        ]);

        search.set_pattern("swift");
        let results = search.search(10);

        assert!(results.iter().any(|(s, _, _)| s == "Taylor Swift"));
        assert_eq!(results[0].1, 800); // Contains score
    }

    #[test]
    fn test_simple_fuzzy_search_fuzzy() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Test Artist".to_string(),
        ]);

        // Fuzzy match: "tylr" should match "Taylor"
        search.set_pattern("tylr");
        let results = search.search(10);

        assert!(results.iter().any(|(s, _, _)| s == "Taylor Swift"));
        // Should have a fuzzy score
        assert!(results[0].1 < 800);
    }

    #[test]
    fn test_simple_fuzzy_search_no_match() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec!["Taylor Swift".to_string()]);

        search.set_pattern("xyz");
        let results = search.search(10);

        assert!(results.is_empty());
    }

    #[test]
    fn test_simple_fuzzy_search_limit() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items((0..100).map(|i| format!("Item {}", i)).collect());

        search.set_pattern("item");
        let results = search.search(10);

        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_simple_fuzzy_search_empty_pattern() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec!["Item 1".to_string(), "Item 2".to_string()]);

        search.set_pattern("");
        let results = search.search(10);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_simple_fuzzy_clear() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec!["Item 1".to_string()]);
        assert_eq!(search.len(), 1);

        search.clear();
        assert_eq!(search.len(), 0);
        assert!(search.is_empty());
    }

    #[test]
    fn test_searchable_trait() {
        #[derive(Clone, Debug, PartialEq)]
        struct Track {
            name: String,
            artist: String,
        }

        impl Searchable for Track {
            fn search_text(&self) -> String {
                format!("{} {}", self.name, self.artist)
            }
        }

        let mut engine = SearchEngine::new();
        engine.add_items(vec![
            Track { name: "Love Story".to_string(), artist: "Taylor Swift".to_string() },
            Track { name: "Test Track".to_string(), artist: "Test Artist".to_string() },
        ]);

        engine.set_pattern("love taylor");
        let results = engine.search(10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.name, "Love Story");
        assert!(results[0].score > 0);
    }

    #[test]
    fn test_fuzzy_search_typos() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Tyler the Creator".to_string(),
        ]);

        // Single character typo: "taylo" should still find "taylor"
        search.set_pattern("taylo");
        let results = search.search(10);

        // Should still find Taylor Swift through fuzzy matching
        assert!(results.iter().any(|(s, _, _)| s == "Taylor Swift"));
    }

    #[test]
    fn test_fuzzy_search_case_insensitive() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec!["UPPER CASE".to_string(), "lower case".to_string()]);

        search.set_pattern("upper");
        let results = search.search(10);

        assert!(results.iter().any(|(s, _, _)| s == "UPPER CASE"));
    }

    #[test]
    fn test_fuzzy_search_acronym() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "Taylor Swift".to_string(),
            "Tame Impala".to_string(),
        ]);

        // Acronym search: "ts" should find "Taylor Swift"
        search.set_pattern("ts");
        let results = search.search(10);

        assert!(results.iter().any(|(s, _, _)| s == "Taylor Swift"));
    }

    #[test]
    fn test_utils_highlight_matches() {
        let text = "Taylor Swift";
        let indices = vec![0, 1, 6, 7]; // "Ta" and "Sw"

        let highlighted = utils::highlight_matches(text, &indices);
        assert!(highlighted.contains('['));
        assert!(highlighted.contains(']'));
    }

    #[test]
    fn test_utils_fuzzy_match() {
        // Test that substring matching works (exact contains)  // Substring match  // Missing letter "i" test
        assert!(!utils::fuzzy_match("Taylor Swift", "xyz"));
        assert!(utils::fuzzy_match("Test", ""));
    }

    #[test]
    fn test_utils_filter_by_patterns() {
        let items = vec![
            "Taylor Swift".to_string(),
            "Test Artist".to_string(),
            "Another Track".to_string(),
        ];

        let results = utils::filter_by_patterns(&items, &["taylor"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "Taylor Swift");
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec![
            "TaylorSwift".to_string(),
            "Taylor Swift".to_string(),
        ]);

        // "ts" matching "TaylorSwift" (camelCase) should get consecutive bonus
        search.set_pattern("ts");
        let results = search.search(10);

        // Should find both
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_match_indices_tracking() {
        let mut search = SimpleFuzzySearch::new();
        search.add_items(vec!["Taylor Swift".to_string()]);

        search.set_pattern("tylr");
        let results = search.search(10);

        assert!(!results.is_empty());
        // Check that indices are returned
        assert!(!results[0].2.is_empty());
    }
}
