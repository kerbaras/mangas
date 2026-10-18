//! Pure eviction plan for the cover cache.

use std::time::SystemTime;

/// One cached file as seen on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    /// Last time the cover was stored or shown.
    pub used: SystemTime,
}

/// Names to delete so that at most `max_files` files and `max_bytes` bytes remain, keeping the
/// most recently used ones.
pub fn to_evict(mut entries: Vec<Entry>, max_files: usize, max_bytes: u64) -> Vec<String> {
    entries.sort_by(|a, b| b.used.cmp(&a.used).then_with(|| a.name.cmp(&b.name)));
    let mut kept_bytes = 0u64;
    let mut kept = 0usize;
    entries
        .into_iter()
        .filter(|e| {
            let keep = kept < max_files && kept_bytes.saturating_add(e.size) <= max_bytes;
            if keep {
                kept += 1;
                kept_bytes += e.size;
            }
            !keep
        })
        .map(|e| e.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn entry(name: &str, size: u64, age: u64) -> Entry {
        Entry {
            name: name.into(),
            size,
            used: SystemTime::UNIX_EPOCH + Duration::from_secs(1000 - age),
        }
    }

    #[test]
    fn keeps_newest_within_file_limit() {
        let all = vec![
            entry("old", 1, 30),
            entry("new", 1, 10),
            entry("mid", 1, 20),
        ];
        assert_eq!(to_evict(all, 2, 100), vec!["old"]);
    }

    #[test]
    fn keeps_newest_within_byte_limit() {
        let all = vec![entry("a", 60, 10), entry("b", 60, 20), entry("c", 30, 30)];
        // "b" does not fit after "a"; the smaller, older "c" still does.
        assert_eq!(to_evict(all, 10, 100), vec!["b"]);
    }

    #[test]
    fn nothing_to_do_under_the_limits() {
        assert!(to_evict(vec![entry("a", 1, 1)], 10, 10).is_empty());
    }
}
