//! Orchestrates the benchmark: read each clip, run every keyed engine, score.

use crate::bench::clips::{self, ClipSpec, EngineSpec};
use crate::bench::engines;
use crate::bench::report::{ClipResult, Record, RecordResult, Report};
use crate::bench::score::{character_error_rate, word_error_rate};
use crate::bench::wav;
use crate::mode::ModeLanguage;
use std::path::Path;

pub async fn run(audio_dir: &Path) -> Result<String, String> {
    let (available, skipped) = partition_by_key(clips::engines());
    if available.is_empty() {
        return Err("no API keys set — export at least one provider key".to_string());
    }

    let mut clip_results = Vec::new();
    for clip in clips::clips() {
        match run_clip(audio_dir, &clip, &available).await {
            Some(result) => clip_results.push(result),
            None => eprintln!("[bench] skipping {} — file not found", clip.stem),
        }
    }

    let report = Report {
        clips: clip_results,
        skipped_engines: skipped,
        priced_engines: available
            .iter()
            .map(|(spec, _)| (spec.label(), spec.usd_per_minute()))
            .collect(),
    };
    Ok(crate::bench::report::render(&report))
}

async fn run_clip(
    audio_dir: &Path,
    clip: &ClipSpec,
    available: &[(EngineSpec, String)],
) -> Option<ClipResult> {
    let path = audio_dir.join(format!("{}.wav", clip.stem));
    let (samples, format) = match wav::read_pcm16(&path) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("[bench] {err}");
            return None;
        }
    };
    let duration_secs = wav::duration_seconds(samples.len(), format);

    let mut records = Vec::new();
    for (spec, key) in available {
        eprintln!("[bench] {} → {}", clip.stem, spec.label());
        let result =
            match engines::run(*spec, key.clone(), &samples, format, (clip.language)()).await {
                Ok(run) => RecordResult::Ok {
                    wer: word_error_rate(clip.reference, &run.transcript),
                    cer: character_error_rate(clip.reference, &run.transcript),
                    latency_ms: run.latency.as_millis(),
                    transcript: run.transcript,
                },
                Err(err) => RecordResult::Failed(err),
            };
        records.push(Record {
            engine: spec.label(),
            result,
        });
    }

    Some(ClipResult {
        stem: clip.stem.to_string(),
        role: clip.role,
        language_label: language_label((clip.language)()),
        reference: clip.reference.to_string(),
        duration_secs,
        records,
    })
}

fn partition_by_key(
    specs: Vec<EngineSpec>,
) -> (Vec<(EngineSpec, String)>, Vec<(&'static str, &'static str)>) {
    let mut available = Vec::new();
    let mut skipped = Vec::new();
    for spec in specs {
        match std::env::var(spec.env_var()) {
            Ok(key) if !key.trim().is_empty() => available.push((spec, key)),
            _ => skipped.push((spec.label(), spec.env_var())),
        }
    }
    (available, skipped)
}

fn language_label(language: ModeLanguage) -> String {
    match language {
        ModeLanguage::Exact { code } => code,
        ModeLanguage::Auto => "auto".to_string(),
        ModeLanguage::Hints { .. } => "hints".to_string(),
    }
}
