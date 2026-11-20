use super::soniox_async::{SonioxAsyncClient, TranscriptionRequest, SONIOX_API_BASE_URL};
use super::store::{write_private, MeetingStatus, MeetingStore};
use super::summary;
use super::summary::MeetingSummarySettings;
use super::track::{pcm_bytes_to_samples, Track};
use super::transcript::{merge_tracks, render, segments_from_tokens, Segment};
use crate::cleanup_invoke::is_credential_configured;
use crate::config::{AiCleanupSettings, Settings};
use std::collections::BTreeSet;
use std::fs;

const PCM_SAMPLE_RATE: u32 = 16_000;
const PCM_CHANNELS: u16 = 1;
/// Soniox context is a recognition hint, not a glossary; past a few dozen
/// terms extra entries add little.
const MAX_CONTEXT_TERMS: usize = 100;

/// Everything processing needs from settings, resolved up front so a missing
/// key is reported when the user presses Start rather than after the call.
pub struct ProcessingConfig {
    soniox_key: String,
    cleanup_settings: AiCleanupSettings,
    summary: MeetingSummarySettings,
    terms: Vec<String>,
}

impl ProcessingConfig {
    pub fn from_settings(settings: &Settings) -> Result<Self, String> {
        let soniox_key = settings
            .soniox_api_key
            .clone()
            .filter(|key| !key.is_empty())
            .ok_or("Meeting notes need a Soniox API key — add one under Speech models.")?;
        let provider = settings.meeting_summary.provider;
        if !is_credential_configured(&settings.ai_cleanup, provider) {
            return Err(format!(
                "Meeting notes are set to use {}, which isn't set up yet — connect it on the \
                 Cleanup page or pick another provider.",
                provider.as_str()
            ));
        }
        Ok(Self {
            soniox_key,
            cleanup_settings: settings.ai_cleanup.clone(),
            summary: settings.meeting_summary.clone(),
            terms: context_terms(settings),
        })
    }
}

fn context_terms(settings: &Settings) -> Vec<String> {
    let unique: BTreeSet<&String> = settings
        .term_sets
        .iter()
        .flat_map(|set| set.entries.iter())
        .collect();
    unique
        .into_iter()
        .take(MAX_CONTEXT_TERMS)
        .cloned()
        .collect()
}

/// Transcribes and summarizes a stopped meeting, recording a Failed status
/// with the reason if any step fails. `on_update` fires after every status
/// change so the UI can refresh.
pub async fn run(store: MeetingStore, id: String, config: ProcessingConfig, on_update: impl Fn()) {
    if let Err(error) = process(&store, &id, &config, &on_update).await {
        eprintln!("[meeting] processing {id} failed: {error}");
        if let Err(save_error) = store.update_status(&id, MeetingStatus::Failed(error)) {
            eprintln!("[meeting] recording failure for {id} failed: {save_error}");
        }
    }
    on_update();
}

async fn process(
    store: &MeetingStore,
    id: &str,
    config: &ProcessingConfig,
    on_update: &impl Fn(),
) -> Result<(), String> {
    store.update_status(id, MeetingStatus::Transcribing)?;
    on_update();
    let segments = transcribe_meeting(store, id, config).await?;
    store.save_segments(id, &segments)?;
    store.update_status(id, MeetingStatus::Summarizing)?;
    on_update();
    let notes = summary::summarize(
        &config.cleanup_settings,
        &config.summary,
        &render(&segments),
    )
    .await?;
    store.save_notes(id, &notes)?;
    let mut meta = store.load_meta(id)?;
    meta.status = MeetingStatus::Ready;
    meta.summary_model = Some(config.summary.model.clone());
    store.save_meta(&meta)
}

async fn transcribe_meeting(
    store: &MeetingStore,
    id: &str,
    config: &ProcessingConfig,
) -> Result<Vec<Segment>, String> {
    let client = SonioxAsyncClient::new(SONIOX_API_BASE_URL, &config.soniox_key);
    let (mic, system) = tokio::join!(
        transcribe_track(store, id, Track::Mic, &client, &config.terms),
        transcribe_track(store, id, Track::System, &client, &config.terms),
    );
    let segments = merge_tracks(mic?, system?);
    if segments.is_empty() {
        return Err("No speech was detected in this recording.".to_string());
    }
    Ok(segments)
}

async fn transcribe_track(
    store: &MeetingStore,
    id: &str,
    track: Track,
    client: &SonioxAsyncClient,
    terms: &[String],
) -> Result<Vec<Segment>, String> {
    let Some(flac) = track_flac(store.clone(), id.to_string(), track).await? else {
        return Ok(Vec::new());
    };
    let request = TranscriptionRequest {
        diarize: track == Track::System,
        terms: terms.to_vec(),
    };
    let tokens = client.transcribe(flac, &request).await?;
    Ok(segments_from_tokens(&tokens, track))
}

/// The FLAC for a track, encoding it from the raw PCM on first use. The PCM is
/// deleted once the FLAC is safely written, so a retry reuses the FLAC.
/// `None` when the track recorded nothing.
async fn track_flac(
    store: MeetingStore,
    id: String,
    track: Track,
) -> Result<Option<Vec<u8>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let flac_path = store.flac_path(&id, track);
        if flac_path.exists() {
            return fs::read(&flac_path)
                .map(Some)
                .map_err(|e| format!("read {flac_path:?}: {e}"));
        }
        let pcm_path = store.pcm_path(&id, track);
        let Ok(pcm) = fs::read(&pcm_path) else {
            return Ok(None);
        };
        if pcm.is_empty() {
            return Ok(None);
        }
        let samples = pcm_bytes_to_samples(&pcm);
        let flac =
            crate::groq_audio::encode_to_flac_16k_mono(&samples, PCM_SAMPLE_RATE, PCM_CHANNELS)?;
        write_private(&flac_path, &flac)?;
        fs::remove_file(&pcm_path).map_err(|e| format!("remove {pcm_path:?}: {e}"))?;
        Ok(Some(flac))
    })
    .await
    .map_err(|e| format!("audio encoding task failed: {e}"))?
}
