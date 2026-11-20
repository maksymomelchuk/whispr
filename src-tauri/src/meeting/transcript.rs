use super::track::Track;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MIC_SPEAKER_LABEL: &str = "You";
pub const SINGLE_REMOTE_SPEAKER_LABEL: &str = "Them";
/// A silence this long starts a new line even when the speaker is unchanged,
/// so lines from the two tracks interleave at a readable granularity.
const SEGMENT_BREAK_GAP_MS: u64 = 2_000;
const MS_PER_SECOND: u64 = 1_000;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3_600;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Token {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    #[serde(default)]
    pub speaker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: String,
    pub text: String,
}

pub fn segments_from_tokens(tokens: &[Token], track: Track) -> Vec<Segment> {
    let labeler = SpeakerLabeler::new(tokens, track);
    let mut segments: Vec<Segment> = Vec::new();
    for token in tokens {
        let speaker = labeler.label(token.speaker.as_deref());
        match segments.last_mut() {
            Some(current) if continues(current, &speaker, token) => {
                current.text.push_str(&token.text);
                current.end_ms = token.end_ms;
            }
            _ => segments.push(Segment {
                start_ms: token.start_ms,
                end_ms: token.end_ms,
                speaker,
                text: token.text.clone(),
            }),
        }
    }
    segments
        .into_iter()
        .map(|segment| Segment {
            text: segment.text.trim().to_string(),
            ..segment
        })
        .filter(|segment| !segment.text.is_empty())
        .collect()
}

fn continues(current: &Segment, speaker: &str, token: &Token) -> bool {
    current.speaker == speaker
        && token.start_ms.saturating_sub(current.end_ms) < SEGMENT_BREAK_GAP_MS
}

/// Everything on the mic is "You". On the system track a lone remote voice is
/// "Them"; several voices keep the diarizer's numbering.
struct SpeakerLabeler {
    track: Track,
    single_remote_speaker: bool,
}

impl SpeakerLabeler {
    fn new(tokens: &[Token], track: Track) -> Self {
        let distinct: BTreeSet<&str> = tokens.iter().filter_map(|t| t.speaker.as_deref()).collect();
        Self {
            track,
            single_remote_speaker: distinct.len() <= 1,
        }
    }

    fn label(&self, speaker: Option<&str>) -> String {
        if self.track == Track::Mic {
            return MIC_SPEAKER_LABEL.to_string();
        }
        match speaker {
            Some(id) if !self.single_remote_speaker => format!("Speaker {id}"),
            _ => SINGLE_REMOTE_SPEAKER_LABEL.to_string(),
        }
    }
}

pub fn merge_tracks(mic: Vec<Segment>, system: Vec<Segment>) -> Vec<Segment> {
    let mut merged: Vec<Segment> = mic.into_iter().chain(system).collect();
    merged.sort_by_key(|segment| segment.start_ms);
    merged
}

pub fn render(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|s| format!("[{}] {}: {}", timestamp(s.start_ms), s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn timestamp(ms: u64) -> String {
    let total_seconds = ms / MS_PER_SECOND;
    let hours = total_seconds / SECONDS_PER_HOUR;
    let minutes = (total_seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let seconds = total_seconds % SECONDS_PER_MINUTE;
    if hours > 0 {
        return format!("{hours}:{minutes:02}:{seconds:02}");
    }
    format!("{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start_ms: u64, speaker: Option<&str>) -> Token {
        Token {
            text: text.to_string(),
            start_ms,
            end_ms: start_ms + 200,
            speaker: speaker.map(str::to_string),
        }
    }

    #[test]
    fn mic_tokens_become_one_line_attributed_to_you() {
        let tokens = [token("Hello", 0, None), token(" there", 300, None)];

        let segments = segments_from_tokens(&tokens, Track::Mic);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker, "You");
        assert_eq!(segments[0].text, "Hello there");
    }

    #[test]
    fn long_pause_starts_a_new_line_for_the_same_speaker() {
        let tokens = [token("First.", 0, None), token(" Second.", 5_000, None)];

        let segments = segments_from_tokens(&tokens, Track::Mic);

        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn single_remote_voice_is_labelled_them() {
        let tokens = [token("Hi", 0, Some("1")), token(" all", 300, Some("1"))];

        let segments = segments_from_tokens(&tokens, Track::System);

        assert_eq!(segments[0].speaker, "Them");
    }

    #[test]
    fn several_remote_voices_keep_diarizer_numbers() {
        let tokens = [token("Hi", 0, Some("1")), token(" Hey", 300, Some("2"))];

        let speakers: Vec<String> = segments_from_tokens(&tokens, Track::System)
            .into_iter()
            .map(|s| s.speaker)
            .collect();

        assert_eq!(speakers, vec!["Speaker 1", "Speaker 2"]);
    }

    #[test]
    fn merged_tracks_are_ordered_by_start_time() {
        let mic = segments_from_tokens(&[token("Second", 2_000, None)], Track::Mic);
        let system = segments_from_tokens(&[token("First", 0, Some("1"))], Track::System);

        let rendered = render(&merge_tracks(mic, system));

        assert_eq!(rendered, "[00:00] Them: First\n[00:02] You: Second");
    }

    #[test]
    fn render_uses_hours_past_the_first_hour() {
        let segments = [Segment {
            start_ms: 3_725_000,
            end_ms: 3_726_000,
            speaker: "You".into(),
            text: "Late".into(),
        }];

        assert_eq!(render(&segments), "[1:02:05] You: Late");
    }
}
