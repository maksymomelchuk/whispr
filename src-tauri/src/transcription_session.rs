use crate::recorder::AudioFormat;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedReceiver;

/// Owns one push-to-talk dictation against a transcription provider.
///
/// The implementation forwards captured audio from `chunks` to the provider
/// for as long as the channel stays open (recorder torn down by PTT
/// release), surfaces interim previews on the existing `transcript-partial`
/// event, and returns the final raw transcript on completion. The returned
/// `Duration` is the user-perceived speak duration — PTT-down to
/// chunk-channel close — and excludes any post-close drain the provider may
/// perform to flush queued finals. The returned transcript is raw: the
/// caller applies replacements so each pipeline stage stays observable in
/// the history trace.
pub trait TranscriptionSession {
    async fn run(
        self,
        app: AppHandle,
        format: AudioFormat,
        chunks: UnboundedReceiver<Vec<i16>>,
    ) -> Result<(String, Duration), String>;
}
