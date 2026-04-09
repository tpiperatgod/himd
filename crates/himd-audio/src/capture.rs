use crate::error::AudioError;
use crate::platform;
use himd_core::types::CaptureResult;

/// Diagnostics about the default input device.
#[derive(Debug)]
pub struct CaptureDiagnostics {
    pub summary: String,
    pub device_name: Option<String>,
    pub ok: bool,
    /// Whether a stream could actually be opened (not just device enumeration).
    pub init_ok: bool,
}

impl CaptureDiagnostics {
    /// Probe the default input device availability.
    pub fn probe() -> Self {
        let diag = platform::probe_input_device();
        Self {
            summary: diag.summary,
            device_name: diag.device_name,
            ok: diag.ok,
            init_ok: diag.init_ok,
        }
    }
}

/// Capture audio from the default microphone with VAD-based auto-stop.
///
/// This is a blocking function — call from `tokio::task::spawn_blocking`.
pub fn capture_once_blocking(max_duration_sec: Option<f64>) -> Result<CaptureResult, AudioError> {
    platform::capture_once_blocking(max_duration_sec)
}

/// Async wrapper around `capture_once_blocking` using `spawn_blocking`.
pub async fn capture_once(max_duration_sec: Option<f64>) -> Result<CaptureResult, AudioError> {
    tokio::task::spawn_blocking(move || capture_once_blocking(max_duration_sec))
        .await
        .map_err(|e| AudioError::Device(format!("capture task panicked: {e}")))?
}
