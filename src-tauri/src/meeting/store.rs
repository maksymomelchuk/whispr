use super::echo::flag_echoes;
use super::speakers::SpeakerNames;
use super::track::Track;
use super::transcript::Segment;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const META_FILE: &str = "meta.json";
const TRANSCRIPT_FILE: &str = "transcript.json";
const NOTES_FILE: &str = "notes.md";
pub const INTERRUPTED_MESSAGE: &str =
    "Whispr quit before this meeting finished processing. Retry to process the saved audio.";
const MEETING_ID_PREFIX: &str = "meeting-";
const TEMP_FILE_SUFFIX: &str = ".tmp";

static MEETING_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_meeting_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let sequence = MEETING_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{MEETING_ID_PREFIX}{millis}-{sequence}")
}

/// Ids arrive from the webview and become directory names, so anything other
/// than the generated shape (e.g. `..`) is rejected before touching the disk.
pub fn validate_meeting_id(id: &str) -> Result<(), String> {
    let well_formed = id.strip_prefix(MEETING_ID_PREFIX).is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '-')
    });
    if !well_formed {
        return Err(format!("Invalid meeting id: {id:?}"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum MeetingStatus {
    Recording,
    Transcribing,
    Summarizing,
    Ready,
    Failed(String),
}

impl MeetingStatus {
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    fn is_in_progress(&self) -> bool {
        matches!(
            self,
            Self::Recording | Self::Transcribing | Self::Summarizing
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingMeta {
    pub id: String,
    pub started_at: i64,
    pub duration_ms: u64,
    pub status: MeetingStatus,
    #[serde(default)]
    pub summary_model: Option<String>,
    /// Why the remote side wasn't recorded, when only the mic track exists.
    #[serde(default)]
    pub system_audio_error: Option<String>,
    /// How long after the mic the call-audio track started, so both tracks
    /// share one clock when merged.
    #[serde(default)]
    pub system_offset_ms: u64,
    /// Names for call-side speaker labels, detected while writing the notes
    /// or set by hand. Transcript lines keep the raw label.
    #[serde(default)]
    pub speaker_names: SpeakerNames,
    /// Names the notes use for more than one speaker; see `SummaryOutput`.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub ambiguous_speaker_names: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Meeting {
    pub meta: MeetingMeta,
    pub segments: Vec<Segment>,
    pub notes: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeetingListItem {
    #[serde(flatten)]
    pub meta: MeetingMeta,
    pub title: Option<String>,
}

/// Notes open with a `# title` line; anything else leaves the meeting untitled.
pub fn notes_title(notes: &str) -> Option<String> {
    let first_line = notes.lines().map(str::trim).find(|line| !line.is_empty())?;
    let title = first_line.strip_prefix("# ")?.trim();
    (!title.is_empty()).then(|| title.to_string())
}

/// On-disk layout: one directory per meeting holding its metadata, the raw
/// and encoded audio of each track, the merged transcript, and the notes.
#[derive(Clone)]
pub struct MeetingStore {
    root: PathBuf,
}

impl MeetingStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn create(&self, meta: &MeetingMeta) -> Result<(), String> {
        let dir = self.meeting_dir(&meta.id);
        fs::create_dir_all(&dir).map_err(|e| format!("create meeting dir {dir:?}: {e}"))?;
        restrict_to_owner(&dir);
        self.save_meta(meta)
    }

    pub fn meeting_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub fn pcm_path(&self, id: &str, track: Track) -> PathBuf {
        self.meeting_dir(id)
            .join(format!("{}.pcm", track.file_stem()))
    }

    pub fn flac_path(&self, id: &str, track: Track) -> PathBuf {
        self.meeting_dir(id)
            .join(format!("{}.flac", track.file_stem()))
    }

    pub fn save_meta(&self, meta: &MeetingMeta) -> Result<(), String> {
        let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
        write_private(&self.meeting_dir(&meta.id).join(META_FILE), json.as_bytes())
    }

    pub fn load_meta(&self, id: &str) -> Result<MeetingMeta, String> {
        read_json(&self.meeting_dir(id).join(META_FILE))
    }

    pub fn update_status(&self, id: &str, status: MeetingStatus) -> Result<MeetingMeta, String> {
        let mut meta = self.load_meta(id)?;
        meta.status = status;
        self.save_meta(&meta)?;
        Ok(meta)
    }

    pub fn list(&self) -> Vec<MeetingMeta> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut metas: Vec<MeetingMeta> = entries
            .flatten()
            .filter_map(|entry| {
                read_json(&entry.path().join(META_FILE))
                    .inspect_err(|e| eprintln!("[meeting] skipping unreadable meeting: {e}"))
                    .ok()
            })
            .collect();
        metas.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        metas
    }

    pub fn list_items(&self) -> Vec<MeetingListItem> {
        self.list()
            .into_iter()
            .map(|meta| {
                let title = read_optional(&self.meeting_dir(&meta.id).join(NOTES_FILE))
                    .inspect_err(|e| eprintln!("[meeting] untitled, notes unreadable: {e}"))
                    .ok()
                    .flatten()
                    .and_then(|notes| notes_title(&notes));
                MeetingListItem { meta, title }
            })
            .collect()
    }

    pub fn load(&self, id: &str) -> Result<Meeting, String> {
        let dir = self.meeting_dir(id);
        let transcript = read_optional(&dir.join(TRANSCRIPT_FILE))?;
        let segments = match transcript {
            Some(json) => {
                serde_json::from_str(&json).map_err(|e| format!("parse transcript of {id}: {e}"))?
            }
            None => Vec::new(),
        };
        let notes = read_optional(&dir.join(NOTES_FILE))?;
        Ok(Meeting {
            meta: self.load_meta(id)?,
            // Re-flagged on every load so transcripts saved before echo
            // detection existed get it too.
            segments: flag_echoes(segments),
            title: notes.as_deref().and_then(notes_title),
            notes,
        })
    }

    pub fn save_segments(&self, id: &str, segments: &[Segment]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(segments).map_err(|e| e.to_string())?;
        write_private(&self.meeting_dir(id).join(TRANSCRIPT_FILE), json.as_bytes())
    }

    pub fn save_notes(&self, id: &str, notes: &str) -> Result<(), String> {
        write_private(&self.meeting_dir(id).join(NOTES_FILE), notes.as_bytes())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let dir = self.meeting_dir(id);
        fs::remove_dir_all(&dir).map_err(|e| format!("delete meeting {dir:?}: {e}"))
    }

    /// Meetings left mid-pipeline by a quit or crash can never finish on their
    /// own; flag them so the user can retry from the audio already on disk.
    pub fn fail_interrupted(&self) -> Result<(), String> {
        for meta in self
            .list()
            .into_iter()
            .filter(|m| m.status.is_in_progress())
        {
            self.update_status(&meta.id, MeetingStatus::Failed(INTERRUPTED_MESSAGE.into()))?;
        }
        Ok(())
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("parse {path:?}: {e}"))
}

/// Missing is a normal state (not transcribed yet); any other failure is not.
fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read {path:?}: {e}")),
    }
}

/// Written to a temp file and renamed into place, so a crash mid-write never
/// leaves a truncated file where a good one was. Owner-only, because meeting
/// audio and transcripts are as sensitive as the call itself.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut temp_name = path.as_os_str().to_owned();
    temp_name.push(TEMP_FILE_SUFFIX);
    let temp_path = PathBuf::from(temp_name);
    fs::write(&temp_path, bytes).map_err(|e| format!("write {temp_path:?}: {e}"))?;
    restrict_to_owner(&temp_path);
    fs::rename(&temp_path, path).map_err(|e| format!("rename {temp_path:?} to {path:?}: {e}"))
}

fn restrict_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, started_at: i64, status: MeetingStatus) -> MeetingMeta {
        MeetingMeta {
            id: id.to_string(),
            started_at,
            duration_ms: 0,
            status,
            summary_model: None,
            system_audio_error: None,
            system_offset_ms: 0,
            speaker_names: SpeakerNames::new(),
            ambiguous_speaker_names: BTreeSet::new(),
        }
    }

    #[test]
    fn generated_meeting_ids_are_unique_and_valid() {
        let first = new_meeting_id();
        let second = new_meeting_id();

        assert_ne!(first, second);
        assert!(validate_meeting_id(&first).is_ok());
    }

    #[test]
    fn ids_that_could_escape_the_store_are_rejected() {
        for id in [
            "..",
            "meeting-../x",
            "meeting-",
            "../meeting-1",
            "meeting-1/2",
        ] {
            assert!(validate_meeting_id(id).is_err(), "{id}");
        }
    }

    #[test]
    fn corrupt_transcript_is_an_error_not_an_empty_meeting() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store
            .create(&meta("meeting-1", 1, MeetingStatus::Ready))
            .unwrap();
        fs::write(
            store.meeting_dir("meeting-1").join(TRANSCRIPT_FILE),
            "{oops",
        )
        .unwrap();

        assert!(store.load("meeting-1").is_err());
    }

    #[test]
    fn list_returns_newest_meeting_first() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store
            .create(&meta("older", 100, MeetingStatus::Ready))
            .unwrap();
        store
            .create(&meta("newer", 200, MeetingStatus::Ready))
            .unwrap();

        let ids: Vec<String> = store.list().into_iter().map(|m| m.id).collect();

        assert_eq!(ids, vec!["newer", "older"]);
    }

    #[test]
    fn fail_interrupted_marks_only_unfinished_meetings_failed() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store
            .create(&meta("recording", 1, MeetingStatus::Recording))
            .unwrap();
        store
            .create(&meta("done", 2, MeetingStatus::Ready))
            .unwrap();

        store.fail_interrupted().unwrap();

        let interrupted = MeetingStatus::Failed(INTERRUPTED_MESSAGE.into());
        assert_eq!(store.load_meta("recording").unwrap().status, interrupted);
        assert_eq!(
            store.load_meta("done").unwrap().status,
            MeetingStatus::Ready
        );
    }

    #[test]
    fn load_includes_saved_notes_and_transcript() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store.create(&meta("m", 1, MeetingStatus::Ready)).unwrap();
        let segment = Segment {
            start_ms: 0,
            end_ms: 900,
            speaker: "You".into(),
            text: "Hello".into(),
            echo: false,
        };
        store.save_segments("m", &[segment]).unwrap();
        store.save_notes("m", "## Summary").unwrap();

        let meeting = store.load("m").unwrap();

        assert_eq!(meeting.segments.len(), 1);
        assert_eq!(meeting.notes.as_deref(), Some("## Summary"));
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

    #[test]
    fn loading_flags_echoes_in_transcripts_saved_without_them() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store.create(&meta("m", 1, MeetingStatus::Ready)).unwrap();
        store
            .save_segments("m", &[line("Them", "Ship it."), line("You", "Ship it.")])
            .unwrap();

        let echoes: Vec<bool> = store
            .load("m")
            .unwrap()
            .segments
            .iter()
            .map(|s| s.echo)
            .collect();

        assert_eq!(echoes, vec![false, true]);
    }

    #[test]
    fn title_comes_from_the_leading_heading_of_the_notes() {
        assert_eq!(
            notes_title("\n# Beta launch sync\n\n## TL;DR\n- ship").as_deref(),
            Some("Beta launch sync")
        );
    }

    #[test]
    fn notes_without_a_leading_heading_are_untitled() {
        assert_eq!(notes_title("## TL;DR\n# Not first"), None);
        assert_eq!(notes_title("#   "), None);
    }

    #[test]
    fn list_items_carry_each_meetings_title() {
        let root = tempfile::tempdir().unwrap();
        let store = MeetingStore::new(root.path().to_path_buf());
        store.create(&meta("titled", 2, MeetingStatus::Ready)).unwrap();
        store
            .create(&meta("recording", 1, MeetingStatus::Recording))
            .unwrap();
        store.save_notes("titled", "# Weekly sync\n- item").unwrap();

        let titles: Vec<Option<String>> =
            store.list_items().into_iter().map(|item| item.title).collect();

        assert_eq!(titles, vec![Some("Weekly sync".to_string()), None]);
    }
}
