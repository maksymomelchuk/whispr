use super::store::MeetingStore;
use super::track::{spawn_track_writer, Track};
use crate::recorder::{AudioFormat, Recorder};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

type TrackWriter = JoinHandle<Result<(), String>>;

/// A meeting that is currently recording. It owns a mic recorder separate
/// from the dictation one, so push-to-talk keeps working during the call.
pub struct ActiveMeeting {
    pub id: String,
    started: Instant,
    mic_recorder: Recorder,
    #[cfg(target_os = "macos")]
    system_capture: Option<crate::system_audio::SystemAudioCapture>,
    writers: Vec<TrackWriter>,
}

pub struct StartedCapture {
    pub meeting: ActiveMeeting,
    /// Set when only the mic is recording because the call audio couldn't be.
    pub system_audio_error: Option<String>,
}

pub async fn start(
    store: &MeetingStore,
    id: &str,
    input_device: Option<String>,
) -> Result<StartedCapture, String> {
    let mic_recorder = Recorder::spawn();
    let mic_writer = start_mic_track(&mic_recorder, input_device, store.pcm_path(id, Track::Mic))
        .await
        .map_err(|e| format!("Microphone recording failed: {e}"))?;
    let mut writers = vec![mic_writer];
    let system = start_system_track(store.pcm_path(id, Track::System));
    let system_audio_error = system.as_ref().err().cloned();
    #[cfg(target_os = "macos")]
    let system_capture = system.ok().map(|(capture, writer)| {
        writers.push(writer);
        capture
    });
    Ok(StartedCapture {
        meeting: ActiveMeeting {
            id: id.to_string(),
            started: Instant::now(),
            mic_recorder,
            #[cfg(target_os = "macos")]
            system_capture,
            writers,
        },
        system_audio_error,
    })
}

async fn start_mic_track(
    recorder: &Recorder,
    input_device: Option<String>,
    path: std::path::PathBuf,
) -> Result<TrackWriter, String> {
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();
    let (format_tx, format_rx) = oneshot::channel();
    recorder.start(input_device, chunk_tx, format_tx);
    let format = format_rx
        .await
        .map_err(|_| "recorder thread exited".to_string())??;
    Ok(spawn_track_writer(chunk_rx, format, path))
}

#[cfg(target_os = "macos")]
fn start_system_track(
    path: std::path::PathBuf,
) -> Result<(crate::system_audio::SystemAudioCapture, TrackWriter), String> {
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();
    let (capture, format): (_, AudioFormat) =
        crate::system_audio::SystemAudioCapture::start(chunk_tx)?;
    Ok((capture, spawn_track_writer(chunk_rx, format, path)))
}

#[cfg(not(target_os = "macos"))]
fn start_system_track(_path: std::path::PathBuf) -> Result<((), TrackWriter), String> {
    Err("Recording call audio is only supported on macOS".to_string())
}

impl ActiveMeeting {
    /// Blocks until both tracks are flushed to disk; returns how long the
    /// meeting recorded.
    pub fn stop(self) -> Result<Duration, String> {
        let duration = self.started.elapsed();
        self.mic_recorder.stop();
        #[cfg(target_os = "macos")]
        drop(self.system_capture);
        for writer in self.writers {
            writer
                .join()
                .map_err(|_| "audio writer thread panicked".to_string())??;
        }
        Ok(duration)
    }
}
