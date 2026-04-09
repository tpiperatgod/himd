use crate::error::AudioError;
use crate::platform;
use std::path::Path;

/// Diagnostics about the default output device.
#[derive(Debug)]
pub struct PlaybackDiagnostics {
    pub summary: String,
    pub device_name: Option<String>,
    pub ok: bool,
    /// Whether a stream could actually be opened (not just device enumeration).
    pub init_ok: bool,
}

impl PlaybackDiagnostics {
    /// Probe the default output device availability.
    pub fn probe() -> Self {
        let diag = platform::probe_output_device();
        Self {
            summary: diag.summary,
            device_name: diag.device_name,
            ok: diag.ok,
            init_ok: diag.init_ok,
        }
    }
}

/// Play a local audio file through the default output device.
/// Blocks until playback completes.
pub fn play_file(path: &Path) -> Result<(), AudioError> {
    platform::play_file(path)
}
