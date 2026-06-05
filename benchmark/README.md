# Speech engine benchmark

Compares every transcription engine on the same recorded clips so you can pick
the right model per Mode. Reports word/character error rate, latency, and an
estimated cost.

## 1. Record the clips

Read each passage in [`passages.md`](./passages.md) once and save it as
`<stem>.wav` (16 kHz mono 16-bit). Recording + conversion instructions are at
the bottom of that file. Drop the WAVs in `benchmark/recordings/` (gitignored)
or any folder you pass on the command line.

## 2. Set provider keys

Each engine runs only if its key is in the environment; others are skipped and
listed in the report.

```sh
export DEEPGRAM_API_KEY=...
export GROQ_API_KEY=...
export ASSEMBLYAI_API_KEY=...
export OPENAI_API_KEY=...
export ELEVENLABS_API_KEY=...
```

## 3. Run

```sh
cd src-tauri
cargo run --features bench --bin bench -- /path/to/recordings > ../benchmark/results.md
```

Progress prints to stderr; the Markdown report goes to stdout (redirect it to a
file as above). The audio path defaults to `benchmark/recordings` if omitted.

## What it measures

- **WER / CER** against the verbatim passage, normalized for case, punctuation,
  and whitespace. Numbers are _not_ canonicalized, so the number/date clip
  (`c_numbers`) and the translation clip (`f_translate`) are presented for a
  transcript eyeball rather than scored.
- **Latency** — wall-clock to the final transcript. Streaming engines are fed
  faster than real time, so this reflects upload + flush, not live finalization.
- **Cost** — audio minutes × a per-minute rate. Rates are hardcoded estimates
  in `src-tauri/src/bench/clips.rs` (`usd_per_minute`); verify them.

## Scope

Cloud engines only: Deepgram, Groq (both Whisper models), AssemblyAI, OpenAI
(`gpt-4o-transcribe` and `-mini`), ElevenLabs. Local Whisper/Parakeet is left
out — it needs downloaded model files and the on-device runtime. No custom
term/correction sets are applied, so this measures base accuracy, not keyterm
support. Both are natural extensions to `src-tauri/src/bench/`.
