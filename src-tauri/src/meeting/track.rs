use crate::groq_audio::downmix_to_mono;
use crate::recorder::AudioFormat;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;

pub const TRACK_SAMPLE_RATE: u32 = 16_000;
/// Device buffers arrive every ~10 ms with some jitter; only a lag beyond this
/// is a real gap in delivery.
const GAP_TOLERANCE_SAMPLES: f64 = TRACK_SAMPLE_RATE as f64 * 0.2;

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

/// Linear resampler that carries its phase from one chunk to the next.
/// Resampling each device buffer on its own drops the fractional output
/// sample at every boundary, which shortens an hour-long track by seconds and
/// lets the two tracks drift apart.
pub struct StreamResampler {
    input_per_output: f64,
    /// Position of the next output sample, in input samples, relative to the
    /// start of the next chunk; -1.0..0.0 means it lies between the previous
    /// chunk's last sample and the next chunk's first.
    position: f64,
    previous_sample: Option<i16>,
}

impl StreamResampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        Self {
            input_per_output: input_rate as f64 / output_rate as f64,
            position: 0.0,
            previous_sample: None,
        }
    }

    pub fn process(&mut self, input: &[i16]) -> Vec<i16> {
        let Some(&last) = input.last() else {
            return Vec::new();
        };
        let sample_at = |index: isize| -> f64 {
            if index < 0 {
                return self.previous_sample.unwrap_or(input[0]) as f64;
            }
            input[index as usize] as f64
        };
        let mut output = Vec::new();
        while self.position.floor() as isize + 1 < input.len() as isize {
            let index = self.position.floor() as isize;
            let fraction = self.position - index as f64;
            let value = sample_at(index) + (sample_at(index + 1) - sample_at(index)) * fraction;
            output.push(value.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16);
            self.position += self.input_per_output;
        }
        self.position -= input.len() as f64;
        self.previous_sample = Some(last);
        output
    }
}

/// A Core Audio tap delivers no buffers while nothing is playing, so the call
/// track would silently drop every pause and slide out of sync with the mic.
/// Tracks samples written against the wall clock and reports the silence to
/// insert so each sample stays at its true time.
pub struct WallClockAligner {
    started: Instant,
    written: u64,
}

impl WallClockAligner {
    pub fn new(started: Instant) -> Self {
        Self {
            started,
            written: 0,
        }
    }

    pub fn silence_before(&mut self, arrived_at: Instant, incoming: usize) -> usize {
        let elapsed = arrived_at
            .saturating_duration_since(self.started)
            .as_secs_f64();
        let expected_end = elapsed * TRACK_SAMPLE_RATE as f64;
        let behind = expected_end - (self.written + incoming as u64) as f64;
        let silence = if behind > GAP_TOLERANCE_SAMPLES {
            behind as usize
        } else {
            0
        };
        self.written += (silence + incoming) as u64;
        silence
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
        let mut resampler = StreamResampler::new(format.sample_rate, TRACK_SAMPLE_RATE);
        let mut aligner = WallClockAligner::new(Instant::now());
        while let Some(chunk) = chunks.blocking_recv() {
            let arrived_at = Instant::now();
            let samples = resampler.process(&downmix_to_mono(&chunk, format.channels));
            let silence = aligner.silence_before(arrived_at, samples.len());
            let pcm: Vec<u8> = std::iter::repeat_n(0i16, silence)
                .chain(samples)
                .flat_map(|sample| sample.to_le_bytes())
                .collect();
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
    use std::time::Duration;

    #[test]
    fn writer_appends_mono_pcm_until_sender_drops() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mic.pcm");
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let format = AudioFormat {
            sample_rate: TRACK_SAMPLE_RATE,
            channels: 2,
        };
        let writer = spawn_track_writer(receiver, format, path.clone());

        sender.send(vec![10, 30, 20, 40]).unwrap();
        sender.send(vec![0, 100]).unwrap();
        sender.send(vec![0, 0]).unwrap();
        drop(sender);
        writer.join().unwrap().unwrap();

        let samples = pcm_bytes_to_samples(&std::fs::read(path).unwrap());
        assert_eq!(samples, vec![20, 30, 50]);
    }

    #[test]
    fn on_time_chunks_get_no_padding() {
        let started = Instant::now();
        let mut aligner = WallClockAligner::new(started);

        let silence = aligner.silence_before(started + Duration::from_millis(10), 160);

        assert_eq!(silence, 0);
    }

    #[test]
    fn a_delivery_gap_is_filled_with_silence_up_to_the_wall_clock() {
        let started = Instant::now();
        let mut aligner = WallClockAligner::new(started);

        let silence = aligner.silence_before(started + Duration::from_secs(3), 160);

        assert_eq!(silence, 3 * TRACK_SAMPLE_RATE as usize - 160);
    }

    #[test]
    fn resampling_odd_sized_chunks_keeps_the_track_duration() {
        let mut resampler = StreamResampler::new(48_000, TRACK_SAMPLE_RATE);
        let chunk = vec![0i16; 512];

        let produced: usize = (0..3_000).map(|_| resampler.process(&chunk).len()).sum();

        let expected = 3_000 * 512 / 3;
        assert!(produced.abs_diff(expected) <= 1, "{produced} vs {expected}");
    }

    #[test]
    fn resampling_interpolates_across_chunk_boundaries() {
        let mut resampler = StreamResampler::new(2, 1);
        let mut output = resampler.process(&[0, 10, 20]);
        output.extend(resampler.process(&[30, 40, 50]));

        assert_eq!(output, vec![0, 20, 40]);
    }
}
