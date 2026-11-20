use crate::groq_audio::to_pcm_16k_mono_bytes;
use crate::recorder::AudioFormat;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread::JoinHandle;
use tokio::sync::mpsc::UnboundedReceiver;

/// The two audio sources of a meeting, kept as separate recordings so each
/// transcript line can be attributed to "you" or to the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Mic,
    System,
}

impl Track {
    pub fn file_stem(self) -> &'static str {
        match self {
            Track::Mic => "mic",
            Track::System => "system",
        }
    }
}

/// Streams a track to disk as 16 kHz mono PCM while it records, so a crash
/// mid-meeting loses at most the last buffered chunk rather than the meeting.
/// The thread ends once the capture drops its sender.
pub fn spawn_track_writer(
    mut chunks: UnboundedReceiver<Vec<i16>>,
    format: AudioFormat,
    path: PathBuf,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open {path:?}: {e}"))?;
        while let Some(chunk) = chunks.blocking_recv() {
            let pcm = to_pcm_16k_mono_bytes(&chunk, format.sample_rate, format.channels)?;
            file.write_all(&pcm)
                .map_err(|e| format!("write {path:?}: {e}"))?;
        }
        file.flush().map_err(|e| format!("flush {path:?}: {e}"))
    })
}

pub fn pcm_bytes_to_samples(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_appends_resampled_mono_pcm_until_sender_drops() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mic.pcm");
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let format = AudioFormat {
            sample_rate: 16_000,
            channels: 1,
        };
        let writer = spawn_track_writer(receiver, format, path.clone());

        sender.send(vec![1, 2, 3]).unwrap();
        sender.send(vec![4]).unwrap();
        drop(sender);
        writer.join().unwrap().unwrap();

        let samples = pcm_bytes_to_samples(&std::fs::read(path).unwrap());
        assert_eq!(samples, vec![1, 2, 3, 4]);
    }
}
