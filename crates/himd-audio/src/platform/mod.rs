//! Platform dispatch for audio capture and playback.
//!
//! Public entrypoints are defined here, with actual implementations in the
//! `macos` and `windows` submodules selected at compile time.

mod macos;
mod windows;

use crate::error::AudioError;
use himd_core::types::CaptureResult;

/// Diagnostics about the default input device.
#[derive(Debug)]
pub struct InputDeviceDiagnostics {
    pub summary: String,
    pub device_name: Option<String>,
    pub ok: bool,
    /// Whether a stream could actually be opened (not just device enumeration).
    pub init_ok: bool,
}

/// Diagnostics about the default output device.
#[derive(Debug)]
pub struct OutputDeviceDiagnostics {
    pub summary: String,
    pub device_name: Option<String>,
    pub ok: bool,
    /// Whether a stream could actually be opened (not just device enumeration).
    pub init_ok: bool,
}

/// Probe the default input device.
pub fn probe_input_device() -> InputDeviceDiagnostics {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        macos::probe_input_device()
    } else if cfg!(target_os = "windows") {
        windows::probe_input_device()
    } else {
        InputDeviceDiagnostics {
            summary: "unsupported platform".into(),
            device_name: None,
            ok: false,
            init_ok: false,
        }
    }
}

/// Probe the default output device.
pub fn probe_output_device() -> OutputDeviceDiagnostics {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        macos::probe_output_device()
    } else if cfg!(target_os = "windows") {
        windows::probe_output_device()
    } else {
        OutputDeviceDiagnostics {
            summary: "unsupported platform".into(),
            device_name: None,
            ok: false,
            init_ok: false,
        }
    }
}

/// Capture audio from the default microphone with VAD-based auto-stop.
pub fn capture_once_blocking(max_duration_sec: Option<f64>) -> Result<CaptureResult, AudioError> {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        macos::capture_once_blocking(max_duration_sec)
    } else if cfg!(target_os = "windows") {
        windows::capture_once_blocking(max_duration_sec)
    } else {
        Err(AudioError::Device("unsupported platform".into()))
    }
}

/// Play a local audio file through the default output device.
pub fn play_file(path: &std::path::Path) -> Result<(), AudioError> {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        macos::play_file(path)
    } else if cfg!(target_os = "windows") {
        windows::play_file(path)
    } else {
        Err(AudioError::Device("unsupported platform".into()))
    }
}
