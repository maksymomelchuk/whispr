use super::transcript::{Segment, MIC_SPEAKER_LABEL, SINGLE_REMOTE_SPEAKER_LABEL};
use std::collections::{BTreeMap, BTreeSet};

pub const SPEAKERS_OPEN_TAG: &str = "<speakers>";
const SPEAKERS_CLOSE_TAG: &str = "</speakers>";
/// Longer than any real name; anything past this is the model rambling.
const MAX_SPEAKER_NAME_CHARS: usize = 60;

/// Transcript label (e.g. "Speaker 1") → the person's name.
pub type SpeakerNames = BTreeMap<String, String>;

pub struct SummaryOutput {
    pub notes: String,
    pub speaker_names: SpeakerNames,
}

/// Splits the model's reply into the notes and the `<speakers>` map it was
/// asked to append. A missing or malformed map only costs the names, so the
/// notes are still kept.
pub fn split_summary_output(reply: &str, segments: &[Segment]) -> SummaryOutput {
    let Some(open) = reply.rfind(SPEAKERS_OPEN_TAG) else {
        return SummaryOutput {
            notes: reply.trim().to_string(),
            speaker_names: SpeakerNames::new(),
        };
    };
    let after_open = &reply[open + SPEAKERS_OPEN_TAG.len()..];
    let json = after_open
        .find(SPEAKERS_CLOSE_TAG)
        .map_or(after_open, |close| &after_open[..close]);
    let speaker_names = match serde_json::from_str::<SpeakerNames>(json.trim()) {
        Ok(names) => accepted_names(names, &remote_labels(segments)),
        Err(error) => {
            eprintln!("[meeting] ignoring unreadable speaker names {json:?}: {error}");
            SpeakerNames::new()
        }
    };
    SummaryOutput {
        notes: reply[..open].trim().to_string(),
        speaker_names,
    }
}

fn remote_labels(segments: &[Segment]) -> BTreeSet<&str> {
    segments
        .iter()
        .map(|segment| segment.speaker.as_str())
        .filter(|speaker| *speaker != MIC_SPEAKER_LABEL)
        .collect()
}

/// Only labels that exist on the call side can be named; "You" stays You. A
/// name given to two labels, or equal to another label, can't be told apart in
/// the notes, so it is dropped.
fn accepted_names(names: SpeakerNames, remote_labels: &BTreeSet<&str>) -> SpeakerNames {
    let candidates: Vec<(String, String)> = names
        .into_iter()
        .filter(|(label, _)| remote_labels.contains(label.as_str()))
        .filter_map(|(label, name)| clean_name(&name).map(|name| (label, name)))
        .filter(|(label, name)| label != name)
        .collect();
    candidates
        .iter()
        .filter(|(label, name)| {
            let shared = candidates
                .iter()
                .any(|(other, other_name)| other != label && same_name(other_name, name));
            let is_a_label = remote_labels.iter().any(|other| same_name(other, name));
            !shared && !is_a_label && !same_name(name, MIC_SPEAKER_LABEL)
        })
        .cloned()
        .collect()
}

pub fn same_name(first: &str, second: &str) -> bool {
    first.to_lowercase() == second.to_lowercase()
}

pub fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let usable = !name.is_empty() && name.chars().count() <= MAX_SPEAKER_NAME_CHARS;
    usable.then(|| name.to_string())
}

/// The notes name speakers the way the transcript showed them, so a manual
/// rename carries over. "Them" is also an everyday word, so notes written
/// under that label are left alone rather than risk rewriting prose.
pub fn rename_in_notes(notes: &str, previous: &str, renamed: &str) -> String {
    if previous == SINGLE_REMOTE_SPEAKER_LABEL || previous == renamed {
        return notes.to_string();
    }
    replace_whole_words(notes, previous, renamed)
}

fn replace_whole_words(text: &str, from: &str, to: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut copied_up_to = 0;
    for (start, _) in text.match_indices(from) {
        let end = start + from.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        let whole_word =
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric);
        if whole_word {
            result.push_str(&text[copied_up_to..start]);
            result.push_str(to);
            copied_up_to = end;
        }
    }
    result.push_str(&text[copied_up_to..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(speaker: &str) -> Segment {
        Segment {
            start_ms: 0,
            end_ms: 1_000,
            speaker: speaker.to_string(),
            text: "Hi".to_string(),
            echo: false,
        }
    }

    fn two_remote_speakers() -> Vec<Segment> {
        vec![segment("You"), segment("Speaker 1"), segment("Speaker 2")]
    }

    #[test]
    fn speakers_block_is_split_from_the_notes() {
        let reply =
            "# Sync\n## TL;DR\n- Ship Friday\n<speakers>{\"Speaker 1\": \"Daniel\"}</speakers>";

        let output = split_summary_output(reply, &two_remote_speakers());

        assert_eq!(output.notes, "# Sync\n## TL;DR\n- Ship Friday");
        assert_eq!(output.speaker_names["Speaker 1"], "Daniel");
    }

    #[test]
    fn reply_without_speakers_block_keeps_notes_and_names_no_one() {
        let output = split_summary_output("# Sync", &two_remote_speakers());

        assert_eq!(output.notes, "# Sync");
        assert!(output.speaker_names.is_empty());
    }

    #[test]
    fn malformed_speakers_block_keeps_the_notes() {
        let output =
            split_summary_output("# Sync\n<speakers>{oops</speakers>", &two_remote_speakers());

        assert_eq!(output.notes, "# Sync");
        assert!(output.speaker_names.is_empty());
    }

    #[test]
    fn the_user_and_unknown_labels_cannot_be_renamed() {
        let reply = r#"# Sync
<speakers>{"You": "Maksym", "Speaker 9": "Ghost", "Speaker 2": "Olena"}</speakers>"#;

        let names = split_summary_output(reply, &two_remote_speakers()).speaker_names;

        assert_eq!(
            names,
            SpeakerNames::from([("Speaker 2".to_string(), "Olena".to_string())])
        );
    }

    #[test]
    fn blank_or_unchanged_names_are_dropped() {
        let reply = r#"<speakers>{"Speaker 1": "  ", "Speaker 2": "Speaker 2"}</speakers>"#;

        let names = split_summary_output(reply, &two_remote_speakers()).speaker_names;

        assert!(names.is_empty());
    }

    #[test]
    fn manual_rename_replaces_the_label_in_notes_as_a_whole_word() {
        let notes = "- [ ] Speaker 1 — send the deck\n- Speaker 12 agreed";

        let renamed = rename_in_notes(notes, "Speaker 1", "Daniel");

        assert_eq!(renamed, "- [ ] Daniel — send the deck\n- Speaker 12 agreed");
    }

    #[test]
    fn manual_rename_leaves_notes_alone_for_the_them_label() {
        let notes = "Them asked us to tell them the date.";

        assert_eq!(rename_in_notes(notes, "Them", "Olena"), notes);
    }

    #[test]
    fn a_name_inside_a_longer_word_is_left_alone() {
        let renamed = rename_in_notes("TomTom navigation, said Tom.", "Tom", "Daniel");

        assert_eq!(renamed, "TomTom navigation, said Daniel.");
    }

    #[test]
    fn a_name_given_to_two_speakers_is_dropped() {
        let reply = r#"<speakers>{"Speaker 1": "Sam", "Speaker 2": "sam"}</speakers>"#;

        let names = split_summary_output(reply, &two_remote_speakers()).speaker_names;

        assert!(names.is_empty());
    }
}
