use crate::models::{DirectoryRecord, SearchResult};
use std::time::{SystemTime, UNIX_EPOCH};

fn subsequence_score(needle: &str, haystack: &str) -> Option<f64> {
    if needle.is_empty() { return Some(0.0); }
    let mut index = 0usize;
    let chars: Vec<char> = needle.chars().collect();
    let mut gaps = 0usize;
    for value in haystack.chars() {
        if chars.get(index).is_some_and(|target| *target == value) { index += 1; } else if index > 0 { gaps += 1; }
    }
    (index == chars.len()).then_some(40.0 - gaps.min(30) as f64)
}

pub fn rank(query: &str, records: &[DirectoryRecord], limit: usize) -> Vec<SearchResult> {
    let needle = query.to_lowercase();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let mut results: Vec<_> = records.iter().filter_map(|record| {
        let name = record.name.to_lowercase();
        let path = record.path.to_lowercase();
        let text_score = if name == needle { 100.0 } else if name.starts_with(&needle) { 70.0 } else if name.contains(&needle) { 55.0 } else { subsequence_score(&needle, &path)? };
        let frequency = (1.0 + record.use_count as f64).ln() * 8.0;
        let recency = record.last_used_at.map(|time| {
            let days = now.saturating_sub(time) as f64 / 86_400.0;
            20.0 / (1.0 + days)
        }).unwrap_or(0.0);
        Some(SearchResult { path: record.path.clone(), name: record.name.clone(), score: text_score + frequency + recency, last_used_at: record.last_used_at, use_count: record.use_count })
    }).collect();
    results.sort_by(|left, right| right.score.total_cmp(&left.score).then_with(|| left.path.cmp(&right.path)));
    results.truncate(limit);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn exact_match_ranks_first() {
        let values = vec![DirectoryRecord { path: "/a/example-web".into(), name: "example-web".into(), use_count: 0, last_used_at: None }, DirectoryRecord { path: "/b/example".into(), name: "example".into(), use_count: 0, last_used_at: None }];
        assert_eq!(rank("example", &values, 10)[0].name, "example");
    }
}
