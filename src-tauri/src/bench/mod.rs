//! Offline speech-engine benchmark. Feeds recorded WAV clips through every
//! keyed engine and reports WER/CER, latency, and estimated cost.
//!
//! Built only under the `bench` feature so it never ships in the app:
//! `cargo run --features bench --bin bench -- <audio_dir>`

mod clips;
mod engines;
mod report;
mod run;
mod score;
mod wav;

pub use run::run;
