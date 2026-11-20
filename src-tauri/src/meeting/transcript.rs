use super::track::Track;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MIC_SPEAKER_LABEL: &str = "You";
pub const SINGLE_REMOTE_SPEAKER_LABEL: &str = "Them";
/// A silence this long starts a new line even when the speaker is unchanged,
/// so lines stay readable during long monologues.
const SEGMENT_BREAK_GAP_MS: u64 = 2_000;
const SENTENCE_ENDINGS: [char; 4] = ['.', '?', '!', '…'];
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
    /// The mic picking up the call audio rather than the user speaking. Kept
    /// on disk so the UI can show it on request; left out of the notes.
    #[serde(default)]
    pub echo: bool,
}

/// One track's tokens, positioned on the meeting clock: `offset_ms` is how
/// long after the mic this track started recording.
pub struct TrackTokens<'a> {
    pub tokens: &'a [Token],
    pub track: Track,
    pub offset_ms: u64,
}

/// Groups each track into sentence-sized lines, then interleaves the lines by
/// start time. Sentence granularity lets a reply from the call land between
/// the mic sentences it answered, without splitting words when the two tracks
/// overlap (e.g. the mic echoing the speakers).
pub fn build_segments(mic: TrackTokens<'_>, system: TrackTokens<'_>) -> Vec<Segment> {
    let mut lines = group_into_lines(label_tokens(mic));
    lines.extend(group_into_lines(label_tokens(system)));
    lines.sort_by_key(|line| line.start_ms);
    lines
}

fn label_tokens(track_tokens: TrackTokens<'_>) -> Vec<Segment> {
    let labeler = SpeakerLabeler::new(track_tokens.tokens, track_tokens.track);
    track_tokens
        .tokens
        .iter()
        .map(|token| Segment {
            start_ms: token.start_ms + track_tokens.offset_ms,
            end_ms: token.end_ms + track_tokens.offset_ms,
            speaker: labeler.label(token.speaker.as_deref()),
            text: token.text.clone(),
            echo: false,
        })
        .collect()
}

fn group_into_lines(labelled: Vec<Segment>) -> Vec<Segment> {
    let mut lines: Vec<Segment> = Vec::new();
    for token in labelled {
        match lines.last_mut() {
            Some(line) if continues(line, &token) => {
                line.text.push_str(&token.text);
                line.end_ms = token.end_ms;
            }
            _ => lines.push(token),
        }
    }
    lines
        .into_iter()
        .map(|line| Segment {
            text: line.text.trim().to_string(),
            ..line
        })
        .filter(|line| !line.text.is_empty())
        .collect()
}

fn continues(line: &Segment, token: &Segment) -> bool {
    let sentence_ended = line.text.trim_end().ends_with(SENTENCE_ENDINGS);
    line.speaker == token.speaker
        && !sentence_ended
        && token.start_ms.saturating_sub(line.end_ms) < SEGMENT_BREAK_GAP_MS
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

/// The transcript the notes are written from: echo lines would make the
/// model credit the user with what someone else said.
pub fn render_without_echoes(segments: &[Segment]) -> String {
    let spoken: Vec<Segment> = segments.iter().filter(|s| !s.echo).cloned().collect();
    render(&spoken)
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

    fn mic(tokens: &[Token]) -> TrackTokens<'_> {
        TrackTokens {
            tokens,
            track: Track::Mic,
            offset_ms: 0,
        }
    }

    fn system(tokens: &[Token], offset_ms: u64) -> TrackTokens<'_> {
        TrackTokens {
            tokens,
            track: Track::System,
            offset_ms,
        }
    }

    #[test]
    fn mic_tokens_become_one_line_attributed_to_you() {
        let tokens = [token("Hel", 0, None), token("lo there", 300, None)];

        let segments = build_segments(mic(&tokens), system(&[], 0));

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker, "You");
        assert_eq!(segments[0].text, "Hello there");
    }

    #[test]
    fn overlapping_tracks_never_split_words() {
        let mic_tokens = [token("Ag", 100, None), token("reed.", 300, None)];
        let system_tokens = [token("Ag", 0, Some("1")), token("reed.", 200, Some("1"))];

        let rendered = render(&build_segments(mic(&mic_tokens), system(&system_tokens, 0)));

        assert_eq!(rendered, "[00:00] Them: Agreed.\n[00:00] You: Agreed.");
    }

    #[test]
    fn long_pause_starts_a_new_line_for_the_same_speaker() {
        let tokens = [token("First.", 0, None), token(" Second.", 5_000, None)];

        let segments = build_segments(mic(&tokens), system(&[], 0));

        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn single_remote_voice_is_labelled_them() {
        let tokens = [token("Hi", 0, Some("1")), token(" all", 300, Some("1"))];

        let segments = build_segments(mic(&[]), system(&tokens, 0));

        assert_eq!(segments[0].speaker, "Them");
    }

    #[test]
    fn several_remote_voices_keep_diarizer_numbers() {
        let tokens = [token("Hi", 0, Some("1")), token(" Hey", 300, Some("2"))];

        let speakers: Vec<String> = build_segments(mic(&[]), system(&tokens, 0))
            .into_iter()
            .map(|s| s.speaker)
            .collect();

        assert_eq!(speakers, vec!["Speaker 1", "Speaker 2"]);
    }

    #[test]
    fn a_reply_from_the_call_splits_the_mic_turn_it_answers() {
        let mic_tokens = [token("Option A.", 0, None), token(" Agreed.", 1_500, None)];
        let system_tokens = [token("Use option B.", 600, Some("1"))];

        let rendered = render(&build_segments(mic(&mic_tokens), system(&system_tokens, 0)));

        assert_eq!(
            rendered,
            "[00:00] You: Option A.\n[00:00] Them: Use option B.\n[00:01] You: Agreed."
        );
    }

    #[test]
    fn system_offset_shifts_call_audio_onto_the_meeting_clock() {
        let system_tokens = [token("Late start", 0, Some("1"))];

        let segments = build_segments(mic(&[]), system(&system_tokens, 3_000));

        assert_eq!(segments[0].start_ms, 3_000);
    }

    #[test]
    fn render_uses_hours_past_the_first_hour() {
        let segments = [Segment {
            start_ms: 3_725_000,
            end_ms: 3_726_000,
            speaker: "You".into(),
            text: "Late".into(),
            echo: false,
        }];

        assert_eq!(render(&segments), "[1:02:05] You: Late");
    }
}
