//! macOS (and Linux) audio backend using cpal + rodio.

use crate::error::AudioError;
use crate::platform::{InputDeviceDiagnostics, OutputDeviceDiagnostics};
use crate::vad::{StopReason, VadConfig, VadState};
use crate::wav;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use himd_core::types::{CaptureResult, StoppedBy};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};

const CHANNELS: u32 = 1;
const MAX_CAP_SEC: f64 = 60.0;

/// Map a VAD StopReason to the MCP StoppedBy enum.
fn map_stop_reason(reason: StopReason) -> StoppedBy {
    match reason {
        StopReason::NoSpeech => StoppedBy::NoSpeech,
        StopReason::Silence => StoppedBy::Silence,
        StopReason::Timeout => StoppedBy::Timeout,
    }
}

/// Process a chunk of i16 audio samples: store, run VAD energy detection.
fn process_capture_chunk(
    data: &[i16],
    samples: &Arc<Mutex<Vec<i16>>>,
    pending: &Arc<Mutex<Vec<i16>>>,
    vad: &Arc<Mutex<VadState>>,
    stop_reason: &Arc<Mutex<Option<StopReason>>>,
    chunk_size: usize,
    sample_rate: u32,
) {
    let mut all = samples.lock().unwrap();
    all.extend_from_slice(data);

    let mut pend = pending.lock().unwrap();
    pend.extend_from_slice(data);

    while pend.len() >= chunk_size {
        let chunk: Vec<i16> = pend.drain(..chunk_size).collect();
        let energy = rms_energy(&chunk);
        let duration_ms = (chunk.len() as u64 * 1000) / sample_rate as u64;
        let mut v = vad.lock().unwrap();
        if let Some(reason) = v.push_energy(energy, duration_ms) {
            let mut s = stop_reason.lock().unwrap();
            if s.is_none() {
                *s = Some(reason);
            }
        }
    }
}

/// Compute RMS energy for a slice of i16 samples.
fn rms_energy(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    ((sum / samples.len() as f64).sqrt() / i16::MAX as f64) as f32
}

/// Probe the default input device on macOS/Linux.
pub fn probe_input_device() -> InputDeviceDiagnostics {
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => {
            let name = device.name().unwrap_or_else(|_| "unknown".into());
            let init_ok = match device.default_input_config() {
                Ok(default_config) => {
                    let sample_format = default_config.sample_format();
                    let config = cpal::StreamConfig {
                        channels: 1,
                        sample_rate: default_config.sample_rate(),
                        buffer_size: cpal::BufferSize::Default,
                    };
                    match sample_format {
                        cpal::SampleFormat::I16 => device
                            .build_input_stream(
                                &config,
                                move |_data: &[i16], _: &cpal::InputCallbackInfo| {},
                                |err| eprintln!("probe input error: {err}"),
                                None,
                            )
                            .is_ok(),
                        cpal::SampleFormat::F32 => device
                            .build_input_stream(
                                &config,
                                move |_data: &[f32], _: &cpal::InputCallbackInfo| {},
                                |err| eprintln!("probe input error: {err}"),
                                None,
                            )
                            .is_ok(),
                        _ => false,
                    }
                }
                Err(_) => false,
            };
            InputDeviceDiagnostics {
                summary: if init_ok {
                    format!("default input device: {name}")
                } else {
                    format!("default input device: {name} (stream init failed)")
                },
                device_name: Some(name),
                ok: true,
                init_ok,
            }
        }
        None => InputDeviceDiagnostics {
            summary: "no default input device found".into(),
            device_name: None,
            ok: false,
            init_ok: false,
        },
    }
}

/// Probe the default output device on macOS/Linux.
pub fn probe_output_device() -> OutputDeviceDiagnostics {
    let host = cpal::default_host();
    match host.default_output_device() {
        Some(device) => {
            let name =
                cpal::traits::DeviceTrait::name(&device).unwrap_or_else(|_| "unknown".into());
            let init_ok = rodio::OutputStream::try_default().is_ok();
            OutputDeviceDiagnostics {
                summary: if init_ok {
                    format!("default output device: {name}")
                } else {
                    format!("default output device: {name} (stream init failed)")
                },
                device_name: Some(name),
                ok: true,
                init_ok,
            }
        }
        None => OutputDeviceDiagnostics {
            summary: "no default output device found".into(),
            device_name: None,
            ok: false,
            init_ok: false,
        },
    }
}

/// Capture audio from the default microphone on macOS/Linux.
///
/// Uses the device's native sample rate (queried via `default_input_config`)
/// rather than hardcoding a rate the device may not support. Audio is converted
/// to mono i16 regardless of the device's native format.
pub fn capture_once_blocking(max_duration_sec: Option<f64>) -> Result<CaptureResult, AudioError> {
    let capped = max_duration_sec
        .map(|d| d.clamp(1.0, MAX_CAP_SEC))
        .unwrap_or(30.0);

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AudioError::Device("no default input device".into()))?;

    // Query the device's actual default config to get native sample rate and format
    let default_config = device
        .default_input_config()
        .map_err(|e| AudioError::Device(format!("failed to query input config: {e}")))?;
    let sample_format = default_config.sample_format();
    let sample_rate = default_config.sample_rate().0;

    let config = cpal::StreamConfig {
        channels: CHANNELS as u16,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = Arc::clone(&samples);

    let vad = Arc::new(Mutex::new(VadState::new(VadConfig {
        max_duration_ms: (capped * 1000.0) as u64,
        ..VadConfig::default()
    })));
    let vad_clone = Arc::clone(&vad);
    let stop_reason: Arc<Mutex<Option<StopReason>>> = Arc::new(Mutex::new(None));
    let stop_clone = Arc::clone(&stop_reason);

    let chunk_size = (sample_rate / 10) as usize;
    let pending: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_clone = Arc::clone(&pending);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let i16_samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    process_capture_chunk(
                        &i16_samples,
                        &samples_clone,
                        &pending_clone,
                        &vad_clone,
                        &stop_clone,
                        chunk_size,
                        sample_rate,
                    );
                },
                |err| eprintln!("audio input error: {err}"),
                None,
            )
            .map_err(|e| AudioError::Stream(format!("failed to build input stream: {e}")))?,
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    process_capture_chunk(
                        data,
                        &samples_clone,
                        &pending_clone,
                        &vad_clone,
                        &stop_clone,
                        chunk_size,
                        sample_rate,
                    );
                },
                |err| eprintln!("audio input error: {err}"),
                None,
            )
            .map_err(|e| AudioError::Stream(format!("failed to build input stream: {e}")))?,
        _ => {
            return Err(AudioError::Stream(format!(
                "unsupported sample format: {sample_format:?}"
            )))
        }
    };

    stream
        .play()
        .map_err(|e| AudioError::Stream(format!("failed to start stream: {e}")))?;

    let start = std::time::Instant::now();
    let max_dur = std::time::Duration::from_secs_f64(capped);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        if start.elapsed() >= max_dur {
            let mut s = stop_reason.lock().unwrap();
            if s.is_none() {
                *s = Some(StopReason::Timeout);
            }
        }

        let reason = stop_reason.lock().unwrap().take();
        if let Some(reason) = reason {
            drop(stream);

            let all_samples = samples.lock().unwrap();
            let epoch_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let output_path = wav::build_capture_output_path(epoch_ms);
            wav::write_wav_mono_i16(&output_path, sample_rate, &all_samples)?;

            let file_size = std::fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0);
            let duration_ms = if all_samples.is_empty() {
                0
            } else {
                (all_samples.len() as u64 * 1000) / sample_rate as u64
            };

            return Ok(CaptureResult {
                temp_audio_path: output_path.to_string_lossy().to_string(),
                format: "wav".to_string(),
                duration_ms,
                sample_rate,
                channels: CHANNELS,
                file_size_bytes: file_size,
                stopped_by: map_stop_reason(reason),
            });
        }
    }
}

/// Play a local audio file through the default output device on macOS/Linux.
pub fn play_file(path: &Path) -> Result<(), AudioError> {
    let file = File::open(path).map_err(|e| {
        AudioError::Io(std::io::Error::new(
            e.kind(),
            format!("failed to open {}: {e}", path.display()),
        ))
    })?;
    let reader = BufReader::new(file);

    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| AudioError::Device(format!("failed to open output stream: {e}")))?;

    let source = Decoder::new(reader)
        .map_err(|e| AudioError::Stream(format!("failed to decode audio: {e}")))?;

    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| AudioError::Stream(format!("failed to create audio sink: {e}")))?;

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
