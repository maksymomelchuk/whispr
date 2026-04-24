use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream};
use std::io::Cursor;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// Look up the saved input device by name. If the saved device no longer
/// exists (unplugged, renamed), warn and fall back to the system default so
/// recording still works.
fn resolve_device(host: &Host, preferred: Option<&str>) -> Option<Device> {
    if let Some(name) = preferred {
        match host.input_devices() {
            Ok(iter) => {
                for d in iter {
                    if d.name().ok().as_deref() == Some(name) {
                        return Some(d);
                    }
                }
                eprintln!(
                    "[recorder] saved input device {name:?} not found; falling back to default"
                );
            }
            Err(e) => eprintln!("[recorder] input_devices enumeration failed: {e}"),
        }
    }
    host.default_input_device()
}

/// cpal::Stream is !Send on macOS (CoreAudio), so we can't store it in shared
/// state or tear it down from an arbitrary thread. Instead we pin a single
/// long-lived audio thread that owns every stream, and send commands over an
/// mpsc channel. Callers hand back a oneshot reply channel for Stop.
enum Cmd {
    Start(Option<String>),
    Stop(mpsc::Sender<Result<Vec<u8>, String>>),
}

#[derive(Clone)]
pub struct Recorder {
    tx: mpsc::Sender<Cmd>,
}

impl Recorder {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        thread::spawn(move || audio_thread(rx));
        Self { tx }
    }

    /// `device` is the saved preference. None (or an unknown name) falls back
    /// to the system default input.
    pub fn start(&self, device: Option<String>) {
        if let Err(e) = self.tx.send(Cmd::Start(device)) {
            eprintln!("[recorder] send Start failed: {e}");
        }
    }

    /// Enumerate current input devices by name. Called from the UI thread
    /// on demand.
    pub fn list_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        match host.input_devices() {
            Ok(iter) => iter.filter_map(|d| d.name().ok()).collect(),
            Err(e) => {
                eprintln!("[recorder] input_devices enumeration failed: {e}");
                Vec::new()
            }
        }
    }

    /// Blocks until the audio thread has torn down the stream and encoded the
    /// captured samples to WAV. Call this from a blocking task — not from a
    /// tokio runtime thread directly.
    pub fn stop(&self) -> Result<Vec<u8>, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(Cmd::Stop(reply_tx))
            .map_err(|e| format!("send Stop: {e}"))?;
        reply_rx
            .recv()
            .map_err(|e| format!("recv Stop reply: {e}"))?
    }
}

fn audio_thread(rx: mpsc::Receiver<Cmd>) {
    let mut session: Option<Session> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start(device) => {
                if session.is_some() {
                    eprintln!("[recorder] Start ignored: already recording");
                    continue;
                }
                match Session::start(device.as_deref()) {
                    Ok(s) => {
                        println!(
                            "[recorder] started ({} Hz, {} ch)",
                            s.sample_rate, s.channels
                        );
                        session = Some(s);
                    }
                    Err(e) => eprintln!("[recorder] Start failed: {e}"),
                }
            }
            Cmd::Stop(reply) => {
                let result = match session.take() {
                    Some(s) => s.finish(),
                    None => Err("Stop with no active session".to_string()),
                };
                let _ = reply.send(result);
            }
        }
    }
}

struct Session {
    _stream: Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
}

impl Session {
    fn start(preferred: Option<&str>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = resolve_device(&host, preferred)
            .ok_or_else(|| "no input device available".to_string())?;
        let supported = device
            .default_input_config()
            .map_err(|e| format!("default_input_config: {e}"))?;

        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_cb = samples.clone();
        let err_cb = |e| eprintln!("[recorder] stream error: {e}");

        let stream = match format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let mut buf = samples_cb.lock().unwrap();
                    buf.extend(
                        data.iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16),
                    );
                },
                err_cb,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let mut buf = samples_cb.lock().unwrap();
                    buf.extend_from_slice(data);
                },
                err_cb,
                None,
            ),
            other => return Err(format!("unsupported sample format: {other:?}")),
        }
        .map_err(|e| format!("build_input_stream: {e}"))?;

        stream.play().map_err(|e| format!("stream.play: {e}"))?;

        Ok(Self {
            _stream: stream,
            samples,
            sample_rate,
            channels,
        })
    }

    /// Dropping `_stream` stops the cpal input. Any tail callbacks racing
    /// teardown will find the mutex locked or lose their writes — we accept
    /// a few ms of trailing audio loss.
    fn finish(self) -> Result<Vec<u8>, String> {
        let sample_rate = self.sample_rate;
        let channels = self.channels;
        drop(self._stream);
        let samples = std::mem::take(
            &mut *self
                .samples
                .lock()
                .map_err(|_| "samples mutex poisoned".to_string())?,
        );
        println!(
            "[recorder] captured {} samples ({:.2}s)",
            samples.len(),
            samples.len() as f32 / (sample_rate as f32 * channels as f32)
        );
        encode_wav(&samples, sample_rate, channels)
    }
}

fn encode_wav(samples: &[i16], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("wav writer: {e}"))?;
        for &s in samples {
            writer
                .write_sample(s)
                .map_err(|e| format!("wav write: {e}"))?;
        }
        writer.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    }
    Ok(cursor.into_inner())
}
