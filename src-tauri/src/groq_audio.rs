//! 16 kHz mono FLAC encoder for Groq's `/openai/v1/audio/transcriptions` ingest.
//!
//! Takes interleaved i16 PCM at an arbitrary input rate and channel count,
//! downmixes to mono, resamples to 16 kHz via linear interpolation, and emits
//! a complete FLAC byte stream. Whisper tolerates the high-frequency aliasing
//! of linear interpolation; if quality issues surface, swap in `rubato`.

use flacenc::component::BitRepr;
use flacenc::error::Verify;

pub const AUDIO_LEVEL_EVENT: &str = "audio-level";
pub const TRANSCRIPT_PARTIAL_EVENT: &str = "transcript-partial";

const TARGET_SAMPLE_RATE: u32 = 16_000;

pub fn compute_level(chunk: &[i16]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = chunk.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_sq / chunk.len() as f64).sqrt() / i16::MAX as f64;
    if rms <= 0.0 {
        return 0.0;
    }
    const FLOOR_DB: f64 = -40.0;
    const CEIL_DB: f64 = -10.0;
    let db = 20.0 * rms.log10();
    ((db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0) as f32
}

pub fn to_pcm_16k_mono_bytes(
    samples: &[i16],
    input_sample_rate: u32,
    input_channels: u16,
) -> Result<Vec<u8>, String> {
    if input_channels == 0 {
        return Err("input_channels must be > 0".into());
    }
    if input_sample_rate == 0 {
        return Err("input_sample_rate must be > 0".into());
    }
    let mono = downmix_to_mono(samples, input_channels);
    let resampled = resample_linear(&mono, input_sample_rate, TARGET_SAMPLE_RATE);
    Ok(resampled.iter().flat_map(|s| s.to_le_bytes()).collect())
}

/// Convert interleaved i16 PCM to 16 kHz mono f32 samples for whisper inference.
///
/// Samples are normalized to [-1.0, 1.0] by dividing by `32768.0` — the absolute
/// magnitude of `i16::MIN`. Dividing by `i16::MAX` (32767) would push `i16::MIN`
/// to ≈ -1.0000305, outside the [-1, 1] range whisper expects.
pub fn to_pcm_16k_mono_f32(
    samples: &[i16],
    input_sample_rate: u32,
    input_channels: u16,
) -> Result<Vec<f32>, String> {
    if input_channels == 0 {
        return Err("input_channels must be > 0".into());
    }
    if input_sample_rate == 0 {
        return Err("input_sample_rate must be > 0".into());
    }
    let mono = downmix_to_mono(samples, input_channels);
    let resampled = resample_linear(&mono, input_sample_rate, TARGET_SAMPLE_RATE);
    Ok(resampled.iter().map(|&s| s as f32 / I16_NORM_DIVISOR).collect())
}

const I16_NORM_DIVISOR: f32 = 32_768.0;

/// Encode interleaved i16 PCM as a 16 kHz mono FLAC byte buffer.
///
/// `samples` is treated as interleaved when `input_channels > 1`. Samples past
/// the last complete frame are discarded.
pub fn encode_to_flac_16k_mono(
    samples: &[i16],
    input_sample_rate: u32,
    input_channels: u16,
) -> Result<Vec<u8>, String> {
    if input_channels == 0 {
        return Err("input_channels must be > 0".into());
    }
    if input_sample_rate == 0 {
        return Err("input_sample_rate must be > 0".into());
    }

    let mono = downmix_to_mono(samples, input_channels);
    let resampled = resample_linear(&mono, input_sample_rate, TARGET_SAMPLE_RATE);
    encode_mono_16k(&resampled)
}

fn downmix_to_mono(interleaved: &[i16], channels: u16) -> Vec<i16> {
    if channels == 1 {
        return interleaved.to_vec();
    }
    let c = channels as usize;
    let frames = interleaved.len() / c;
    let mut out = Vec::with_capacity(frames);
    for f in 0..frames {
        let base = f * c;
        let mut sum: i32 = 0;
        for ch in 0..c {
            sum += interleaved[base + ch] as i32;
        }
        out.push((sum / c as i32) as i16);
    }
    out
}

fn resample_linear(input: &[i16], in_rate: u32, out_rate: u32) -> Vec<i16> {
    if input.is_empty() {
        return Vec::new();
    }
    if in_rate == out_rate {
        return input.to_vec();
    }

    let in_len = input.len();
    let out_len = (in_len as u64 * out_rate as u64 / in_rate as u64) as usize;
    let mut out = Vec::with_capacity(out_len);

    let ratio = in_rate as f64 / out_rate as f64;
    for j in 0..out_len {
        let t = j as f64 * ratio;
        let i = t as usize;
        let frac = t - i as f64;
        let s = if i + 1 < in_len {
            let a = input[i] as f64;
            let b = input[i + 1] as f64;
            a + (b - a) * frac
        } else {
            input[in_len - 1] as f64
        };
        out.push(s.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16);
    }
    out
}

fn encode_mono_16k(samples: &[i16]) -> Result<Vec<u8>, String> {
    let samples_i32: Vec<i32> = samples.iter().map(|&s| s as i32).collect();

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, e)| format!("flacenc config: {e:?}"))?;

    let source = flacenc::source::MemSource::from_samples(
        &samples_i32,
        1,
        16,
        TARGET_SAMPLE_RATE as usize,
    );

    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| format!("flacenc encode: {e:?}"))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| format!("flacenc write: {e:?}"))?;
    Ok(sink.as_slice().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_sine(freq_hz: f64, sample_rate: u32, channels: u16, secs: f64) -> Vec<i16> {
        let frames = (sample_rate as f64 * secs) as usize;
        let mut out = Vec::with_capacity(frames * channels as usize);
        for i in 0..frames {
            let t = i as f64 / sample_rate as f64;
            let v = (t * freq_hz * std::f64::consts::TAU).sin();
            let s = (v * 30_000.0) as i16;
            for _ in 0..channels {
                out.push(s);
            }
        }
        out
    }

    #[test]
    fn round_trip_48k_stereo_sine() {
        let input_rate = 48_000u32;
        let input_channels = 2u16;
        let duration_secs = 1.0;
        let input = synth_sine(440.0, input_rate, input_channels, duration_secs);

        let bytes = encode_to_flac_16k_mono(&input, input_rate, input_channels)
            .expect("encode succeeds");

        let mut reader = claxon::FlacReader::new(&bytes[..]).expect("FlacReader::new");
        let info = reader.streaminfo();
        assert_eq!(info.sample_rate, TARGET_SAMPLE_RATE);
        assert_eq!(info.channels, 1);
        assert_eq!(info.bits_per_sample, 16);

        let mut decoded = 0usize;
        for s in reader.samples() {
            s.expect("sample decode");
            decoded += 1;
        }

        let expected = (TARGET_SAMPLE_RATE as f64 * duration_secs) as usize;
        let frame_tolerance = info.max_block_size as usize;
        let diff = decoded.abs_diff(expected);
        assert!(
            diff <= frame_tolerance,
            "decoded {decoded} samples vs expected {expected} (diff {diff}, tolerance {frame_tolerance})"
        );
    }

    #[test]
    fn downmix_averages_stereo() {
        let interleaved = [100i16, 200, -100, 200, 0, 0];
        let mono = downmix_to_mono(&interleaved, 2);
        assert_eq!(mono, vec![150, 50, 0]);
    }

    #[test]
    fn resample_passthrough_when_rates_match() {
        let input: Vec<i16> = (0..100).collect();
        let out = resample_linear(&input, 16_000, 16_000);
        assert_eq!(out, input);
    }

    #[test]
    fn resample_48k_to_16k_decimates_by_three() {
        let input: Vec<i16> = (0..48).collect();
        let out = resample_linear(&input, 48_000, 16_000);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 3);
        assert_eq!(out[2], 6);
    }

    #[test]
    fn rejects_zero_channels() {
        let r = encode_to_flac_16k_mono(&[0i16; 16], 48_000, 0);
        assert!(r.is_err());
    }

    #[test]
    fn f32_rejects_zero_channels() {
        assert!(to_pcm_16k_mono_f32(&[0i16; 16], 48_000, 0).is_err());
    }

    #[test]
    fn f32_rejects_zero_sample_rate() {
        assert!(to_pcm_16k_mono_f32(&[0i16; 16], 0, 1).is_err());
    }

    #[test]
    fn f32_samples_are_normalized() {
        let input: Vec<i16> = vec![i16::MAX, i16::MIN, 0];
        let out = to_pcm_16k_mono_f32(&input, 16_000, 1).unwrap();
        for &s in &out {
            assert!(s >= -1.0 && s <= 1.0, "sample {s} out of [-1, 1]");
        }
    }

    #[test]
    fn f32_resamples_48k_mono_to_16k() {
        let input = synth_sine(440.0, 48_000, 1, 1.0);
        let out = to_pcm_16k_mono_f32(&input, 48_000, 1).unwrap();
        let expected_len = 16_000usize;
        let diff = out.len().abs_diff(expected_len);
        assert!(diff <= 1, "expected ~{expected_len} samples, got {}", out.len());
    }

    #[test]
    fn f32_passthrough_when_already_16k_mono() {
        let input: Vec<i16> = vec![100, -100, 0, 200];
        let out = to_pcm_16k_mono_f32(&input, 16_000, 1).unwrap();
        assert_eq!(out.len(), input.len());
        assert!((out[0] - 100.0 / I16_NORM_DIVISOR).abs() < 1e-4);
        assert!((out[1] - (-100.0 / I16_NORM_DIVISOR)).abs() < 1e-4);
        assert_eq!(out[2], 0.0);
    }

    #[test]
    fn f32_downmixes_stereo_then_normalizes() {
        let input: Vec<i16> = vec![100, 200, -100, 200];
        let out = to_pcm_16k_mono_f32(&input, 16_000, 2).unwrap();
        assert_eq!(out.len(), 2);
        assert!((out[0] - 150.0 / I16_NORM_DIVISOR).abs() < 1e-4);
        assert!((out[1] - 50.0 / I16_NORM_DIVISOR).abs() < 1e-4);
    }
}
