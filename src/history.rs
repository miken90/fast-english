//! Recent results, so a result survives the popup that showed it.
//!
//! Pure data and policy: what an entry holds, how many are kept, and how long
//! each one may be. Reading and writing the file is `persistence::HistoryStore`.

use serde::{Deserialize, Serialize};

/// How many results are kept. The oldest falls off the end.
pub const MAX_ENTRIES: usize = 20;

/// Longest source or result text stored per entry, in characters.
///
/// History is a convenience, not an archive: this bounds a file that would
/// otherwise grow with whatever the user happens to paste. At 20 entries with
/// both fields full the file stays under about a megabyte even for text that is
/// entirely multi-byte.
pub const MAX_TEXT_CHARS: usize = 8000;

/// One completed result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Tool id (`fix-grammar`, `translate`, …) — restores the right tool.
    pub tool_id: String,
    /// Tool name as it was shown, including translate directions.
    pub tool_label: String,
    pub source_text: String,
    pub output: String,
    /// Unix seconds. Stored as a number so the view can format it locally.
    pub timestamp: u64,
    /// Translate language ids, so reopening rebuilds the pair that actually
    /// ran rather than the default one. `None` for every other tool, and for
    /// entries written before this was stored.
    #[serde(default)]
    pub translate_from: Option<String>,
    #[serde(default)]
    pub translate_to: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    /// Newest first, so the list renders in order and index 0 is the last result.
    entries: Vec<HistoryEntry>,
}

impl History {
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Add a completed result, applying both caps.
    ///
    /// `translate_langs` carries the from/to language ids for a translation and
    /// is `None` for every other tool.
    pub fn record(
        &mut self,
        tool_id: &str,
        tool_label: &str,
        source_text: &str,
        output: &str,
        translate_langs: Option<(&str, &str)>,
    ) {
        self.entries.insert(
            0,
            HistoryEntry {
                tool_id: tool_id.to_string(),
                tool_label: tool_label.to_string(),
                source_text: truncate_chars(source_text, MAX_TEXT_CHARS),
                output: truncate_chars(output, MAX_TEXT_CHARS),
                timestamp: now_unix_seconds(),
                translate_from: translate_langs.map(|(from, _)| from.to_string()),
                translate_to: translate_langs.map(|(_, to)| to.to_string()),
            },
        );
        self.entries.truncate(MAX_ENTRIES);
    }

    /// Drop anything a file on disk should not be trusted to have kept within
    /// bounds — an edited or older file could hold more or longer entries.
    pub fn enforce_limits(&mut self) {
        self.entries.truncate(MAX_ENTRIES);
        for entry in &mut self.entries {
            entry.source_text = truncate_chars(&entry.source_text, MAX_TEXT_CHARS);
            entry.output = truncate_chars(&entry.output, MAX_TEXT_CHARS);
        }
    }
}

/// Cut at a character boundary, marking that something was cut.
///
/// Byte slicing would panic in the middle of a Vietnamese character, and this
/// text is exactly where those appear.
fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((idx, _)) => format!("{}…", &text[..idx]),
        None => text.to_string(),
    }
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_n(history: &mut History, n: usize) {
        for i in 0..n {
            history.record(
                "fix-grammar",
                "Fix Grammar",
                &format!("source {}", i),
                "out",
                None,
            );
        }
    }

    #[test]
    fn a_translation_stores_the_language_pair_it_ran_with() {
        let mut history = History::default();
        history.record(
            "translate",
            "Translate English → Vietnamese",
            "Good morning",
            "Chào buổi sáng",
            Some(("en", "vi")),
        );

        let entry = &history.entries()[0];
        assert_eq!(entry.translate_from.as_deref(), Some("en"));
        assert_eq!(entry.translate_to.as_deref(), Some("vi"));
    }

    #[test]
    fn other_tools_store_no_language_pair() {
        let mut history = History::default();
        history.record("fix-grammar", "Fix Grammar", "She go", "She goes", None);

        let entry = &history.entries()[0];
        assert!(entry.translate_from.is_none());
        assert!(entry.translate_to.is_none());
    }

    #[test]
    fn an_entry_saved_before_languages_were_stored_still_loads() {
        // A file from the previous build: no translate_from/translate_to keys.
        let json = r#"{"entries":[{"tool_id":"translate","tool_label":"Translate Auto Detect → English","source_text":"Xin chào","output":"Hello","timestamp":1}]}"#;

        let history: History = serde_json::from_str(json).expect("old entries must still parse");

        assert_eq!(history.entries().len(), 1);
        assert!(history.entries()[0].translate_from.is_none());
    }

    #[test]
    fn the_newest_result_is_first() {
        let mut history = History::default();
        record_n(&mut history, 3);

        assert_eq!(history.entries()[0].source_text, "source 2");
        assert_eq!(history.entries()[2].source_text, "source 0");
    }

    #[test]
    fn only_the_last_twenty_are_kept() {
        let mut history = History::default();
        record_n(&mut history, MAX_ENTRIES + 15);

        assert_eq!(history.entries().len(), MAX_ENTRIES);
        // The oldest fell off; the newest survived.
        assert_eq!(history.entries()[0].source_text, "source 34");
        assert!(!history
            .entries()
            .iter()
            .any(|e| e.source_text == "source 0"));
    }

    #[test]
    fn an_over_long_entry_is_truncated_and_marked() {
        let mut history = History::default();
        let long = "a".repeat(MAX_TEXT_CHARS + 500);
        history.record("enhance-writing", "Enhance Writing", &long, &long, None);

        let entry = &history.entries()[0];
        assert_eq!(entry.source_text.chars().count(), MAX_TEXT_CHARS + 1);
        assert!(entry.source_text.ends_with('…'));
        assert!(entry.output.ends_with('…'));
    }

    #[test]
    fn truncation_does_not_split_a_vietnamese_character() {
        let mut history = History::default();
        // Every character is multi-byte, so a byte-based cut would panic.
        let text = "ế".repeat(MAX_TEXT_CHARS + 10);
        history.record("translate", "Translate", &text, &text, Some(("en", "vi")));

        let entry = &history.entries()[0];
        assert!(entry.source_text.starts_with('ế'));
        assert_eq!(entry.source_text.chars().count(), MAX_TEXT_CHARS + 1);
    }

    #[test]
    fn text_within_the_cap_is_stored_whole() {
        let mut history = History::default();
        history.record(
            "fix-grammar",
            "Fix Grammar",
            "She go home.",
            "She goes home.",
            None,
        );

        let entry = &history.entries()[0];
        assert_eq!(entry.source_text, "She go home.");
        assert_eq!(entry.output, "She goes home.");
        assert!(!entry.output.ends_with('…'));
    }

    #[test]
    fn limits_are_reapplied_to_whatever_was_on_disk() {
        // A file written by hand, or by an older build with different caps.
        let mut history = History {
            entries: (0..MAX_ENTRIES + 5)
                .map(|i| HistoryEntry {
                    tool_id: "fix-grammar".to_string(),
                    tool_label: "Fix Grammar".to_string(),
                    source_text: "x".repeat(MAX_TEXT_CHARS + 100),
                    output: format!("out {}", i),
                    timestamp: 1,
                    translate_from: None,
                    translate_to: None,
                })
                .collect(),
        };

        history.enforce_limits();

        assert_eq!(history.entries().len(), MAX_ENTRIES);
        assert!(history.entries()[0].source_text.ends_with('…'));
    }

    #[test]
    fn clearing_removes_everything() {
        let mut history = History::default();
        record_n(&mut history, 5);
        history.clear();

        assert!(history.entries().is_empty());
        assert!(history.get(0).is_none());
    }
}
