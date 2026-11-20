use super::transcript::{Segment, MIC_SPEAKER_LABEL};
use std::collections::HashMap;

/// Speaker-to-mic delay is milliseconds, but each track's token timings drift
/// by up to about a second against the other.
const ECHO_TIME_TOLERANCE_MS: u64 = 1_500;
/// The mic hears the call through the room, so its copy is often misheard;
/// most words matching is enough to call it an echo.
const ECHO_MIN_MATCHED_WORD_SHARE: f64 = 0.6;
/// An echo is heard while the call line plays; the user's own reply to it
/// starts once the line is over, so the mic line must sit inside the call span.
const ECHO_SPAN_SLACK_MS: u64 = 1_000;
/// Normalized (apostrophes dropped). A mic line that adds one of these to the
/// call's words says the opposite, so it is the user speaking, not an echo.
const NEGATION_WORDS: [&str; 19] = [
    "not",
    "no",
    "never",
    "dont",
    "doesnt",
    "didnt",
    "isnt",
    "arent",
    "wasnt",
    "werent",
    "cant",
    "cannot",
    "wont",
    "shouldnt",
    "wouldnt",
    "couldnt",
    "не",
    "ні",
    "ніколи",
];

/// Flags a "You" line as an echo when most of its words were said on the call
/// audio at the same time — the mic picking up the speakers, not the user.
pub fn flag_echoes(segments: Vec<Segment>) -> Vec<Segment> {
    let call_lines: Vec<CallLine> = segments
        .iter()
        .filter(|segment| segment.speaker != MIC_SPEAKER_LABEL)
        .map(CallLine::new)
        .collect();
    segments
        .into_iter()
        .map(|segment| {
            let echo =
                segment.speaker == MIC_SPEAKER_LABEL && is_echo_of_call(&segment, &call_lines);
            Segment { echo, ..segment }
        })
        .collect()
}

struct CallLine {
    start_ms: u64,
    end_ms: u64,
    words: Vec<String>,
}

impl CallLine {
    fn new(segment: &Segment) -> Self {
        Self {
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            words: normalized_words(&segment.text),
        }
    }

    fn overlaps(&self, segment: &Segment) -> bool {
        self.start_ms <= segment.end_ms + ECHO_TIME_TOLERANCE_MS
            && segment.start_ms <= self.end_ms + ECHO_TIME_TOLERANCE_MS
    }
}

fn is_echo_of_call(mic_line: &Segment, call_lines: &[CallLine]) -> bool {
    let mic_words = normalized_words(&mic_line.text);
    let overlapping: Vec<&CallLine> = call_lines
        .iter()
        .filter(|line| line.overlaps(mic_line))
        .collect();
    if mic_words.is_empty() || !within_call_span(mic_line, &overlapping) {
        return false;
    }
    let mut call_word_counts: HashMap<&str, usize> = HashMap::new();
    for word in overlapping.iter().flat_map(|line| &line.words) {
        *call_word_counts.entry(word).or_default() += 1;
    }
    let unmatched: Vec<&String> = mic_words
        .iter()
        .filter(|word| !take_word(&mut call_word_counts, word))
        .collect();
    let matched_share = 1.0 - unmatched.len() as f64 / mic_words.len() as f64;
    let adds_negation = unmatched
        .iter()
        .any(|word| NEGATION_WORDS.contains(&word.as_str()));
    matched_share >= ECHO_MIN_MATCHED_WORD_SHARE && !adds_negation
}

fn within_call_span(mic_line: &Segment, overlapping: &[&CallLine]) -> bool {
    let Some(span_start) = overlapping.iter().map(|line| line.start_ms).min() else {
        return false;
    };
    let span_end = overlapping
        .iter()
        .map(|line| line.end_ms)
        .max()
        .unwrap_or(span_start);
    mic_line.start_ms + ECHO_SPAN_SLACK_MS >= span_start
        && mic_line.end_ms <= span_end + ECHO_SPAN_SLACK_MS
}

fn take_word(counts: &mut HashMap<&str, usize>, word: &str) -> bool {
    match counts.get_mut(word) {
        Some(count) if *count > 0 => {
            *count -= 1;
            true
        }
        _ => false,
    }
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(speaker: &str, start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment {
            start_ms,
            end_ms,
            speaker: speaker.to_string(),
            text: text.to_string(),
            echo: false,
        }
    }

    fn echo_flags(segments: Vec<Segment>) -> Vec<bool> {
        flag_echoes(segments).into_iter().map(|s| s.echo).collect()
    }

    #[test]
    fn mic_line_repeating_overlapping_call_line_is_an_echo() {
        let flags = echo_flags(vec![
            line(
                "Speaker 1",
                1_000,
                4_000,
                "Thanks, Daniel. Let's ship on Friday.",
            ),
            line("You", 1_100, 4_100, "thanks daniel lets ship on friday"),
        ]);

        assert_eq!(flags, vec![false, true]);
    }

    #[test]
    fn partially_misheard_echo_still_counts() {
        let flags = echo_flags(vec![
            line("Them", 0, 3_000, "The budget review moves to next Tuesday."),
            line(
                "You",
                200,
                3_200,
                "The budget we view moves to next Tuesday.",
            ),
        ]);

        assert!(flags[1]);
    }

    #[test]
    fn same_words_at_a_different_time_are_not_an_echo() {
        let flags = echo_flags(vec![
            line("Them", 0, 2_000, "Let's ship on Friday."),
            line("You", 10_000, 12_000, "Let's ship on Friday."),
        ]);

        assert!(!flags[1]);
    }

    #[test]
    fn overlapping_mic_line_with_different_words_is_kept() {
        let flags = echo_flags(vec![
            line("Them", 0, 3_000, "Can you send the deck?"),
            line("You", 500, 2_500, "Sure, I will send it tonight."),
        ]);

        assert!(!flags[1]);
    }

    #[test]
    fn echo_split_across_two_call_lines_is_matched() {
        let flags = echo_flags(vec![
            line("Speaker 1", 0, 1_500, "Olena here."),
            line("Speaker 2", 1_600, 3_000, "Hi Olena."),
            line("You", 100, 3_100, "Olena here. Hi Olena."),
        ]);

        assert!(flags[2]);
    }

    #[test]
    fn call_audio_lines_are_never_flagged() {
        let flags = echo_flags(vec![
            line("You", 0, 2_000, "Hello everyone"),
            line("Them", 0, 2_000, "Hello everyone"),
        ]);

        assert_eq!(flags, vec![true, false]);
    }

    #[test]
    fn matching_is_case_and_punctuation_insensitive_for_non_latin_text() {
        let flags = echo_flags(vec![
            line("Them", 0, 2_000, "Дякую, Олено!"),
            line("You", 0, 2_000, "дякую олено"),
        ]);

        assert!(flags[1]);
    }

    #[test]
    fn overlapping_disagreement_is_not_an_echo() {
        let flags = echo_flags(vec![
            line("Them", 0, 3_000, "We should ship on Friday."),
            line("You", 200, 3_100, "We should not ship on Friday."),
        ]);

        assert!(!flags[1]);
    }

    #[test]
    fn reply_that_starts_after_the_call_line_ends_is_not_an_echo() {
        let flags = echo_flags(vec![
            line("Them", 0, 2_000, "Let's ship on Friday."),
            line("You", 3_200, 5_000, "Let's ship on Monday."),
        ]);

        assert!(!flags[1]);
    }
}
