use super::speakers::{clean_name, rename_in_notes, same_name};
use super::store::{Meeting, MeetingStatus, MeetingStore};
use super::transcript::MIC_SPEAKER_LABEL;
use std::collections::BTreeSet;

impl MeetingStore {
    /// Notes are restored if the metadata write fails, so the two files never
    /// disagree about a speaker's name.
    pub fn rename_speaker(&self, id: &str, label: &str, name: &str) -> Result<(), String> {
        let meeting = self.load(id)?;
        if meeting.meta.status != MeetingStatus::Ready {
            return Err(format!(
                "Speakers can be renamed once meeting {id} has finished processing"
            ));
        }
        let renamed = validated_rename(&meeting, label, name)?;
        let mut meta = meeting.meta;
        let previous = meta
            .speaker_names
            .get(label)
            .cloned()
            .unwrap_or_else(|| label.to_string());
        if renamed == label {
            meta.speaker_names.remove(label);
        } else {
            meta.speaker_names
                .insert(label.to_string(), renamed.clone());
        }
        let Some(notes) = meeting.notes else {
            return self.save_meta(&meta);
        };
        self.save_notes(id, &rename_in_notes(&notes, &previous, &renamed))?;
        self.save_meta(&meta)
            .map_err(|error| self.restore_notes(id, &notes, error))
    }

    fn restore_notes(&self, id: &str, notes: &str, error: String) -> String {
        match self.save_notes(id, notes) {
            Ok(()) => error,
            Err(restore_error) => {
                format!("{error}; restoring the notes of {id} also failed: {restore_error}")
            }
        }
    }
}

/// The name `label` will show after the rename: the label itself when `name`
/// is blank. Names already used by another speaker are refused, because the
/// notes could no longer tell the two apart.
fn validated_rename(meeting: &Meeting, label: &str, name: &str) -> Result<String, String> {
    let remote_labels: BTreeSet<&str> = meeting
        .segments
        .iter()
        .map(|segment| segment.speaker.as_str())
        .filter(|speaker| *speaker != MIC_SPEAKER_LABEL)
        .collect();
    if !remote_labels.contains(label) {
        return Err(format!("{label:?} is not a call speaker in this meeting"));
    }
    let renamed = match clean_name(name) {
        Some(cleaned) => cleaned,
        None if name.trim().is_empty() => return Ok(label.to_string()),
        None => return Err(format!("Speaker name is too long: {name:?}")),
    };
    let names = &meeting.meta.speaker_names;
    let taken = same_name(&renamed, MIC_SPEAKER_LABEL)
        || meeting
            .meta
            .ambiguous_speaker_names
            .iter()
            .any(|ambiguous| same_name(ambiguous, &renamed))
        || remote_labels
            .iter()
            .filter(|other| **other != label)
            .any(|other| {
                same_name(other, &renamed)
                    || names.get(*other).is_some_and(|n| same_name(n, &renamed))
            });
    if taken {
        return Err(format!("{renamed:?} is already another speaker's name"));
    }
    Ok(renamed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::speakers::SpeakerNames;
    use crate::meeting::store::MeetingMeta;
    use crate::meeting::transcript::Segment;

    fn meta(id: &str) -> MeetingMeta {
        MeetingMeta {
            id: id.to_string(),
            started_at: 1,
            duration_ms: 0,
            status: MeetingStatus::Ready,
            summary_model: None,
            system_audio_error: None,
            system_offset_ms: 0,
            speaker_names: SpeakerNames::new(),
            ambiguous_speaker_names: BTreeSet::new(),
        }
    }

    fn line(speaker: &str, text: &str) -> Segment {
        Segment {
            start_ms: 0,
            end_ms: 900,
            speaker: speaker.into(),
            text: text.into(),
            echo: false,
        }
    }

    fn ready_meeting_with_speaker_two(store: &MeetingStore) {
        store.create(&meta("m")).unwrap();
        store
            .save_segments("m", &[line("Speaker 2", "Olena here.")])
            .unwrap();
        store
            .save_notes("m", "- [ ] Speaker 2 — share the draft")
            .unwrap();
    }

    #[test]
    fn renaming_a_speaker_stores_the_name_and_updates_the_notes() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);

        store.rename_speaker("m", "Speaker 2", " Olena ").unwrap();

        let meeting = store.load("m").unwrap();
        assert_eq!(meeting.meta.speaker_names["Speaker 2"], "Olena");
        assert_eq!(
            meeting.notes.as_deref(),
            Some("- [ ] Olena — share the draft")
        );
    }

    #[test]
    fn blank_rename_restores_the_label_everywhere() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);
        store.rename_speaker("m", "Speaker 2", "Olena").unwrap();

        store.rename_speaker("m", "Speaker 2", "").unwrap();

        let meeting = store.load("m").unwrap();
        assert!(meeting.meta.speaker_names.is_empty());
        assert_eq!(
            meeting.notes.as_deref(),
            Some("- [ ] Speaker 2 — share the draft")
        );
    }

    #[test]
    fn the_user_cannot_be_renamed() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);

        assert!(store.rename_speaker("m", "You", "Maksym").is_err());
    }

    #[test]

    fn a_name_already_used_by_another_speaker_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);
        store
            .save_segments(
                "m",
                &[line("Speaker 1", "Hi."), line("Speaker 2", "Olena here.")],
            )
            .unwrap();
        store.rename_speaker("m", "Speaker 2", "Olena").unwrap();

        assert!(store.rename_speaker("m", "Speaker 1", "olena").is_err());
    }

    #[test]
    fn speakers_cannot_be_renamed_before_the_notes_are_written() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);
        store
            .update_status("m", MeetingStatus::Summarizing)
            .unwrap();

        assert!(store.rename_speaker("m", "Speaker 2", "Olena").is_err());
    }

    #[test]
    fn a_name_the_notes_use_for_two_speakers_cannot_be_given_to_one() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        ready_meeting_with_speaker_two(&store);
        let mut meta = store.load_meta("m").unwrap();
        meta.ambiguous_speaker_names = BTreeSet::from(["Sam".to_string()]);
        store.save_meta(&meta).unwrap();

        assert!(store.rename_speaker("m", "Speaker 2", "Sam").is_err());
    }
}
