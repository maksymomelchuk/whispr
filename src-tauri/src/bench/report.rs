//! Renders benchmark results as a Markdown report.

use crate::bench::clips::Role;
use std::collections::BTreeMap;
use std::fmt::Write;

pub enum RecordResult {
    Ok {
        transcript: String,
        wer: f64,
        cer: f64,
        latency_ms: u128,
    },
    Failed(String),
}

pub struct Record {
    pub engine: &'static str,
    pub result: RecordResult,
}

pub struct ClipResult {
    pub stem: String,
    pub role: Role,
    pub language_label: String,
    pub reference: String,
    pub duration_secs: f64,
    pub records: Vec<Record>,
}

pub struct Report {
    pub clips: Vec<ClipResult>,
    pub skipped_engines: Vec<(&'static str, &'static str)>,
    pub priced_engines: Vec<(&'static str, f64)>,
}

pub fn render(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("# Speech engine benchmark\n\n");
    render_caveats(&mut out, report);
    for clip in &report.clips {
        render_clip(&mut out, clip);
    }
    render_language_summary(&mut out, report);
    render_cost_summary(&mut out, report);
    out
}

fn render_caveats(out: &mut String, report: &Report) {
    out.push_str("> Scores normalize case, punctuation, and whitespace. Numbers are NOT canonicalized — number/date clips are scored for the transcript eyeball, not WER.\n");
    out.push_str("> Latency is measured from the last audio chunk (≈ PTT release) to the final transcript: finalization lag for streaming engines (fed at real time), full upload+process for batch engines.\n");
    out.push_str("> No custom term/correction sets are configured, so keyterm support is not exercised. Costs are editable estimates.\n\n");
    if report.skipped_engines.is_empty() {
        return;
    }
    out.push_str("**Skipped (no API key):** ");
    let skipped: Vec<String> = report
        .skipped_engines
        .iter()
        .map(|(label, var)| format!("{label} (set `{var}`)"))
        .collect();
    out.push_str(&skipped.join(", "));
    out.push_str("\n\n");
}

fn render_clip(out: &mut String, clip: &ClipResult) {
    let _ = writeln!(
        out,
        "## {} — {}, {}, {:.1}s\n",
        clip.stem,
        clip.language_label,
        clip.role.label(),
        clip.duration_secs
    );
    let _ = writeln!(out, "Reference: _{}_\n", clip.reference);

    if clip.role == Role::Scored {
        render_scored_table(out, clip);
    } else {
        render_latency_table(out, clip);
    }
    render_transcripts(out, clip);
}

fn render_scored_table(out: &mut String, clip: &ClipResult) {
    let best = best_wer(clip);
    out.push_str("| Engine | WER | CER | Latency |\n|---|---|---|---|\n");
    for record in &clip.records {
        match &record.result {
            RecordResult::Ok {
                wer,
                cer,
                latency_ms,
                ..
            } => {
                let marker = if best == Some(*wer) { " ⭐" } else { "" };
                let _ = writeln!(
                    out,
                    "| {}{} | {:.1}% | {:.1}% | {}ms |",
                    record.engine,
                    marker,
                    wer * 100.0,
                    cer * 100.0,
                    latency_ms
                );
            }
            RecordResult::Failed(err) => {
                let _ = writeln!(out, "| {} | — | — | {} |", record.engine, short(err));
            }
        }
    }
    out.push('\n');
}

fn render_latency_table(out: &mut String, clip: &ClipResult) {
    out.push_str("| Engine | Latency |\n|---|---|\n");
    for record in &clip.records {
        match &record.result {
            RecordResult::Ok { latency_ms, .. } => {
                let _ = writeln!(out, "| {} | {}ms |", record.engine, latency_ms);
            }
            RecordResult::Failed(err) => {
                let _ = writeln!(out, "| {} | {} |", record.engine, short(err));
            }
        }
    }
    out.push('\n');
}

fn render_transcripts(out: &mut String, clip: &ClipResult) {
    out.push_str("Transcripts:\n");
    for record in &clip.records {
        match &record.result {
            RecordResult::Ok { transcript, .. } => {
                let _ = writeln!(out, "- **{}**: {}", record.engine, transcript);
            }
            RecordResult::Failed(err) => {
                let _ = writeln!(out, "- **{}**: _failed: {}_", record.engine, short(err));
            }
        }
    }
    out.push('\n');
}

fn render_language_summary(out: &mut String, report: &Report) {
    out.push_str("## Summary — mean WER by language (scored clips)\n\n");
    let mut by_language: BTreeMap<String, BTreeMap<&str, Vec<f64>>> = BTreeMap::new();
    for clip in &report.clips {
        if clip.role != Role::Scored {
            continue;
        }
        let lang_bucket = by_language.entry(clip.language_label.clone()).or_default();
        for record in &clip.records {
            if let RecordResult::Ok { wer, .. } = &record.result {
                lang_bucket.entry(record.engine).or_default().push(*wer);
            }
        }
    }

    for (language, engines) in &by_language {
        let _ = writeln!(out, "**{language}**\n");
        let mut means: Vec<(&str, f64)> = engines
            .iter()
            .map(|(engine, wers)| (*engine, wers.iter().sum::<f64>() / wers.len() as f64))
            .collect();
        means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (rank, (engine, mean)) in means.iter().enumerate() {
            let marker = if rank == 0 { " ⭐ best" } else { "" };
            let _ = writeln!(out, "- {}: {:.1}%{}", engine, mean * 100.0, marker);
        }
        out.push('\n');
    }
}

fn render_cost_summary(out: &mut String, report: &Report) {
    let total_minutes: f64 = report.clips.iter().map(|c| c.duration_secs).sum::<f64>() / 60.0;
    let _ = writeln!(
        out,
        "## Cost (estimate) — {:.2} min of audio across all clips\n",
        total_minutes
    );
    out.push_str("| Engine | USD/min | Est. suite cost |\n|---|---|---|\n");
    for (label, per_minute) in &report.priced_engines {
        let _ = writeln!(
            out,
            "| {} | ${:.4} | ${:.4} |",
            label,
            per_minute,
            per_minute * total_minutes
        );
    }
    out.push_str("\n_Pricing is hardcoded in `clips.rs::usd_per_minute` — verify against current provider rates._\n");
}

fn best_wer(clip: &ClipResult) -> Option<f64> {
    clip.records
        .iter()
        .filter_map(|r| match &r.result {
            RecordResult::Ok { wer, .. } => Some(*wer),
            RecordResult::Failed(_) => None,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn short(error: &str) -> String {
    let trimmed: String = error.chars().take(80).collect();
    trimmed.replace('\n', " ")
}
