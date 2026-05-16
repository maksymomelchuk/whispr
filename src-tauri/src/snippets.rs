use crate::config::SnippetEntry;

/// Expands snippet triggers in `text`, then resolves placeholders in the result.
///
/// Trigger matching is exact, case-sensitive substring replacement (v1).
/// All occurrences of each trigger are replaced.
/// Placeholder resolution runs after all trigger substitutions, so an expansion
/// like "Today is {{DATE}}" correctly resolves the date.
pub fn expand_snippets(text: &str, entries: &[SnippetEntry]) -> String {
    let mut result = text.to_string();
    for entry in entries {
        if entry.trigger.is_empty() {
            continue;
        }
        if result.contains(entry.trigger.as_str()) {
            result = result.replace(entry.trigger.as_str(), &entry.expansion);
        }
    }
    resolve_placeholders(result)
}

fn resolve_placeholders(mut text: String) -> String {
    if text.contains("{{DATE}}") {
        text = text.replace("{{DATE}}", &current_date());
    }
    if text.contains("{{TIME}}") {
        text = text.replace("{{TIME}}", &current_time());
    }
    if text.contains("{{CLIPBOARD}}") {
        text = text.replace("{{CLIPBOARD}}", &read_clipboard());
    }
    text
}

fn current_date() -> String {
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!("{:04}-{:02}-{:02}", now.year(), now.month() as u8, now.day())
}

fn current_time() -> String {
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!("{:02}:{:02}", now.hour(), now.minute())
}

#[cfg(target_os = "macos")]
fn read_clipboard() -> String {
    use std::process::Command;
    Command::new("pbpaste")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn read_clipboard() -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, trigger: &str, expansion: &str) -> SnippetEntry {
        SnippetEntry {
            id: id.to_string(),
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
        }
    }

    #[test]
    fn empty_entries_returns_text_unchanged() {
        assert_eq!(expand_snippets("hello world", &[]), "hello world");
    }

    #[test]
    fn trigger_replaced_with_expansion() {
        let entries = [entry("1", "[email]", "user@example.com")];
        assert_eq!(
            expand_snippets("my email is [email].", &entries),
            "my email is user@example.com."
        );
    }

    #[test]
    fn all_occurrences_of_trigger_are_replaced() {
        let entries = [entry("1", "foo", "bar")];
        assert_eq!(
            expand_snippets("foo and foo and foo", &entries),
            "bar and bar and bar"
        );
    }

    #[test]
    fn empty_trigger_is_skipped() {
        let entries = [entry("1", "", "something")];
        assert_eq!(expand_snippets("hello", &entries), "hello");
    }

    #[test]
    fn multiple_snippets_applied_in_order() {
        let entries = [entry("1", "greet", "Hello"), entry("2", "name", "World")];
        assert_eq!(expand_snippets("greet name", &entries), "Hello World");
    }

    #[test]
    fn no_matching_trigger_returns_text_unchanged() {
        let entries = [entry("1", "[trigger]", "expansion")];
        assert_eq!(expand_snippets("no match here", &entries), "no match here");
    }

    #[test]
    fn trigger_matching_is_case_sensitive() {
        let entries = [entry("1", "[Date]", "2026-01-01")];
        assert_eq!(
            expand_snippets("[date] and [Date]", &entries),
            "[date] and 2026-01-01"
        );
    }

    #[test]
    fn date_placeholder_resolves_to_iso_format() {
        let entries = [entry("1", "[d]", "{{DATE}}")];
        let result = expand_snippets("[d]", &entries);
        // YYYY-MM-DD: 10 chars
        assert_eq!(result.len(), 10, "date should be 10 chars, got: {result}");
        let parts: Vec<&str> = result.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4, "year should be 4 digits");
        assert_eq!(parts[1].len(), 2, "month should be 2 digits");
        assert_eq!(parts[2].len(), 2, "day should be 2 digits");
    }

    #[test]
    fn time_placeholder_resolves_to_hhmm_format() {
        let entries = [entry("1", "[t]", "{{TIME}}")];
        let result = expand_snippets("[t]", &entries);
        // HH:MM: 5 chars
        assert_eq!(result.len(), 5, "time should be 5 chars, got: {result}");
        let parts: Vec<&str> = result.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 2, "hour should be 2 digits");
        assert_eq!(parts[1].len(), 2, "minute should be 2 digits");
    }

    #[test]
    fn clipboard_placeholder_is_replaced() {
        let entries = [entry("1", "[clip]", "{{CLIPBOARD}}")];
        let result = expand_snippets("[clip]", &entries);
        // The trigger was expanded — {{CLIPBOARD}} should not appear literally.
        assert!(
            !result.contains("{{CLIPBOARD}}"),
            "{{CLIPBOARD}} should have been resolved, got: {result}"
        );
    }

    #[test]
    fn expansion_containing_placeholder_resolves() {
        let entries = [entry("1", "today", "Today is {{DATE}}")];
        let result = expand_snippets("today!", &entries);
        assert!(
            result.starts_with("Today is "),
            "should start with 'Today is', got: {result}"
        );
        assert!(result.ends_with('!'));
        assert!(
            !result.contains("{{DATE}}"),
            "{{DATE}} should have been resolved"
        );
    }

    #[test]
    fn multiple_placeholders_in_one_expansion() {
        let entries = [entry("1", "[stamp]", "{{DATE}} {{TIME}}")];
        let result = expand_snippets("[stamp]", &entries);
        assert!(!result.contains("{{DATE}}"));
        assert!(!result.contains("{{TIME}}"));
        // Result should be "YYYY-MM-DD HH:MM"
        assert_eq!(result.len(), 16, "expected 16 chars (date + space + time), got: {result}");
    }
}
