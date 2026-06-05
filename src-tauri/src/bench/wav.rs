//! Minimal RIFF/WAVE reader for 16-bit PCM clips.
//!
//! Walks the chunk list rather than assuming a fixed 44-byte header — afconvert
//! emits a padded `data` offset, so a hardcoded header size would read garbage.

use crate::recorder::AudioFormat;
use std::path::Path;

pub fn read_pcm16(path: &Path) -> Result<(Vec<i16>, AudioFormat), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("{}: not a RIFF/WAVE file", path.display()));
    }

    let mut format: Option<AudioFormat> = None;
    let mut bits_per_sample = 0u16;
    let mut data: Option<&[u8]> = None;
    let mut cursor = 12;

    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size = u32::from_le_bytes(read4(&bytes, cursor + 4)) as usize;
        let body_start = cursor + 8;
        let body_end = body_start.saturating_add(chunk_size).min(bytes.len());

        match chunk_id {
            b"fmt " if chunk_size >= 16 => {
                let channels = u16::from_le_bytes([bytes[body_start + 2], bytes[body_start + 3]]);
                let sample_rate = u32::from_le_bytes(read4(&bytes, body_start + 4));
                bits_per_sample =
                    u16::from_le_bytes([bytes[body_start + 14], bytes[body_start + 15]]);
                format = Some(AudioFormat {
                    sample_rate,
                    channels,
                });
            }
            b"data" => data = Some(&bytes[body_start..body_end]),
            _ => {}
        }
        cursor = body_end + (chunk_size & 1);
    }

    let format = format.ok_or_else(|| format!("{}: missing fmt chunk", path.display()))?;
    if bits_per_sample != 16 {
        return Err(format!(
            "{}: expected 16-bit PCM, got {bits_per_sample}-bit",
            path.display()
        ));
    }
    let data = data.ok_or_else(|| format!("{}: missing data chunk", path.display()))?;
    let samples = data
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Ok((samples, format))
}

pub fn duration_seconds(sample_count: usize, format: AudioFormat) -> f64 {
    let frames = sample_count as f64 / format.channels.max(1) as f64;
    frames / format.sample_rate.max(1) as f64
}

fn read4(bytes: &[u8], offset: usize) -> [u8; 4] {
    [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]
}
