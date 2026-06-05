//! Entrypoint for the speech-engine benchmark.
//!
//! Usage: `cargo run --features bench --bin bench -- <audio_dir>`
//! Reads `<audio_dir>/{a_english,b_technical,...}.wav` and prints a Markdown
//! report to stdout (progress goes to stderr, so `> results.md` captures only
//! the report). Provider keys come from DEEPGRAM_API_KEY, GROQ_API_KEY,
//! ASSEMBLYAI_API_KEY, OPENAI_API_KEY, ELEVENLABS_API_KEY.

use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let audio_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("benchmark/recordings"));

    match whispr_lib::bench::run(&audio_dir).await {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("bench failed: {err}");
            ExitCode::FAILURE
        }
    }
}
