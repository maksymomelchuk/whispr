use super::store::{MeetingStatus, MeetingStore};

const TITLE_PREFIX: &str = "# ";

impl MeetingStore {
    /// The title lives in the notes' leading heading, so copied notes and the
    /// meeting list always agree on it.
    pub fn retitle(&self, id: &str, title: &str) -> Result<(), String> {
        let title = validated_title(title)?;
        let meeting = self.load(id)?;
        let notes = match (&meeting.meta.status, meeting.notes) {
            (MeetingStatus::Ready, Some(notes)) => notes,
            _ => {
                return Err(format!(
                    "Meeting {id} can be renamed once its notes are written"
                ))
            }
        };
        self.save_notes(id, &with_title(&notes, &title))
    }
}

fn validated_title(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("A meeting title can't be blank".to_string());
    }
    if title.contains(['\n', '\r']) {
        return Err(format!("A meeting title must be one line: {title:?}"));
    }
    Ok(title.to_string())
}

/// Replaces the leading `# ` heading, or adds one when the notes have none.
fn with_title(notes: &str, title: &str) -> String {
    let heading = format!("{TITLE_PREFIX}{title}");
    let body = notes.trim_start();
    let Some(first_line) = body.lines().next() else {
        return heading;
    };
    if !first_line.starts_with(TITLE_PREFIX) {
        return format!("{heading}\n\n{body}");
    }
    format!("{heading}{}", &body[first_line.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::store::MeetingMeta;

    fn meta(status: MeetingStatus) -> MeetingMeta {
        MeetingMeta {
            id: "m".to_string(),
            started_at: 1,
            duration_ms: 0,
            status,
            summary_model: None,
            system_audio_error: None,
            system_offset_ms: 0,
            speaker_names: Default::default(),
            ambiguous_speaker_names: Default::default(),
        }
    }

    fn store_with_notes(status: MeetingStatus, notes: &str) -> (tempfile::TempDir, MeetingStore) {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store.create(&meta(status)).unwrap();
        store.save_notes("m", notes).unwrap();
        (root, store)
    }

    #[test]
    fn renaming_replaces_the_leading_heading() {
        let (_root, store) =
            store_with_notes(MeetingStatus::Ready, "# Weekly sync\n\n## TL;DR\n- ship");

        store.retitle("m", "  Launch review ").unwrap();

        let meeting = store.load("m").unwrap();
        assert_eq!(meeting.title.as_deref(), Some("Launch review"));
        assert_eq!(
            meeting.notes.as_deref(),
            Some("# Launch review\n\n## TL;DR\n- ship")
        );
    }

    #[test]
    fn renaming_untitled_notes_adds_a_heading() {
        let (_root, store) = store_with_notes(MeetingStatus::Ready, "## TL;DR\n- ship");

        store.retitle("m", "Launch review").unwrap();

        assert_eq!(
            store.load("m").unwrap().notes.as_deref(),
            Some("# Launch review\n\n## TL;DR\n- ship")
        );
    }

    #[test]
    fn a_blank_title_is_refused() {
        let (_root, store) = store_with_notes(MeetingStatus::Ready, "# Weekly sync");

        assert!(store.retitle("m", "   ").is_err());
        assert_eq!(
            store.load("m").unwrap().title.as_deref(),
            Some("Weekly sync")
        );
    }

    #[test]
    fn a_multi_line_title_is_refused() {
        let (_root, store) = store_with_notes(MeetingStatus::Ready, "# Weekly sync");

        assert!(store.retitle("m", "One\n## Two").is_err());
    }

    #[test]
    fn a_meeting_still_being_summarized_cannot_be_renamed() {
        let (_root, store) = store_with_notes(MeetingStatus::Summarizing, "# Draft");

        assert!(store.retitle("m", "Launch review").is_err());
    }
}
