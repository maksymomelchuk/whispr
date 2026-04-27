use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream};
use std::sync::mpsc;
use std::thread;
use tokio::sync::{mpsc as tokio_mpsc, oneshot};

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

#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// cpal::Stream is !Send on macOS (CoreAudio), so we can't store it in shared
/// state or tear it down from an arbitrary thread. Instead we pin a single
/// long-lived audio thread that owns every stream, and send commands over an
/// mpsc channel.
enum Cmd {
    Start {
        device: Option<String>,
        chunk_tx: tokio_mpsc::UnboundedSender<Vec<i16>>,
        format_tx: oneshot::Sender<Result<AudioFormat, String>>,
    },
    Stop,
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

    /// `format_tx` resolves once the cpal stream is live (or with an error
    /// if setup fails). `stop` drops the stream, which drops `chunk_tx` —
    /// consumers must treat the channel close as end-of-stream.
    pub fn start(
        &self,
        device: Option<String>,
        chunk_tx: tokio_mpsc::UnboundedSender<Vec<i16>>,
        format_tx: oneshot::Sender<Result<AudioFormat, String>>,
    ) {
        if let Err(e) = self.tx.send(Cmd::Start {
            device,
            chunk_tx,
            format_tx,
        }) {
            eprintln!("[recorder] send Start failed: {e}");
        }
    }

    /// Tear down the active capture. Fire-and-forget: end-of-stream is
    /// signaled to the consumer via the chunk channel closing.
    pub fn stop(&self) {
        if let Err(e) = self.tx.send(Cmd::Stop) {
            eprintln!("[recorder] send Stop failed: {e}");
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
}

fn audio_thread(rx: mpsc::Receiver<Cmd>) {
    let mut session: Option<Session> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start {
                device,
                chunk_tx,
                format_tx,
            } => {
                if session.is_some() {
                    eprintln!("[recorder] Start ignored: already recording");
                    let _ = format_tx.send(Err("already recording".into()));
                    continue;
                }
                match Session::start(device.as_deref(), chunk_tx) {
                    Ok((s, format)) => {
                        let _ = format_tx.send(Ok(format));
                        session = Some(s);
                    }
                    Err(e) => {
                        let _ = format_tx.send(Err(e));
                    }
                }
            }
            Cmd::Stop => {
                // Tail samples racing teardown either lose their write or
                // land just before the drop — accepted.
                session = None;
            }
        }
    }
}

struct Session {
    _stream: Stream,
}

impl Session {
    fn start(
        preferred: Option<&str>,
        chunk_tx: tokio_mpsc::UnboundedSender<Vec<i16>>,
    ) -> Result<(Self, AudioFormat), String> {
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

        let err_cb = |e| eprintln!("[recorder] stream error: {e}");

        let stream = match format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let chunk: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    let _ = chunk_tx.send(chunk);
                },
                err_cb,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let _ = chunk_tx.send(data.to_vec());
                },
                err_cb,
                None,
            ),
            other => return Err(format!("unsupported sample format: {other:?}")),
        }
        .map_err(|e| format!("build_input_stream: {e}"))?;

        stream.play().map_err(|e| format!("stream.play: {e}"))?;

        Ok((
            Self { _stream: stream },
            AudioFormat {
                sample_rate,
                channels,
            },
        ))
    }
}
