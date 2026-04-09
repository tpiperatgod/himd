//! Runtime path helpers.
//!
//! All persistent state (captures, TTS output, voice profile) lives
//! under `<CWD>/.voice-bridge/`.

use std::path::PathBuf;

/// Return the runtime base directory for himd state files.
///
/// Resolves to `<CWD>/.voice-bridge/`. Creates the directory if absent.
pub fn runtime_base_dir() -> PathBuf {
    let dir = std::env::current_dir()
        .expect("failed to get current working directory")
        .join(".voice-bridge");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Return the captures subdirectory.
pub fn captures_dir() -> PathBuf {
    runtime_base_dir().join("captures")
}

/// Return the TTS output path for a given timestamp.
pub fn tts_output_path(timestamp_ms: u128) -> PathBuf {
    runtime_base_dir()
        .join("tts")
        .join(format!("{timestamp_ms}.wav"))
}

/// Return the voice profile file path.
pub fn voice_profile_path() -> PathBuf {
    runtime_base_dir().join("voice-profile.json")
}
