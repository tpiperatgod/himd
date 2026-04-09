use himd_core::runtime_paths::{runtime_base_dir, tts_output_path, voice_profile_path};
use std::path::PathBuf;

/// RAII guard that restores CWD on drop, even if a panic occurs.
struct CwdGuard {
    original: PathBuf,
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

/// Helper: run a closure with CWD set to a temp dir, restoring original CWD on drop.
/// Returns the canonical temp path (resolving symlinks like /var -> /private/var on macOS).
fn with_temp_cwd<F, R>(f: F) -> R
where
    F: FnOnce(PathBuf) -> R,
{
    let temp = tempfile::tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    let _guard = CwdGuard { original };
    std::env::set_current_dir(temp.path()).unwrap();
    // Use current_dir() to get the canonicalized path (resolves symlinks).
    let canonical_temp = std::env::current_dir().unwrap();
    f(canonical_temp)
}

#[test]
fn runtime_base_dir_is_cwd_dot_voice_bridge() {
    with_temp_cwd(|temp| {
        let base = runtime_base_dir();
        assert_eq!(base, temp.join(".voice-bridge"));
    });
}

#[test]
fn runtime_base_dir_creates_directory() {
    with_temp_cwd(|_temp| {
        let base = runtime_base_dir();
        assert!(base.exists(), ".voice-bridge should be auto-created");
    });
}

#[test]
fn captures_dir_is_under_base() {
    with_temp_cwd(|temp| {
        let caps = himd_core::runtime_paths::captures_dir();
        assert_eq!(caps, temp.join(".voice-bridge").join("captures"));
    });
}

#[test]
fn voice_profile_path_is_under_base() {
    with_temp_cwd(|temp| {
        let profile = voice_profile_path();
        assert_eq!(
            profile,
            temp.join(".voice-bridge").join("voice-profile.json")
        );
    });
}

#[test]
fn tts_output_path_is_under_base() {
    with_temp_cwd(|temp| {
        let tts = tts_output_path(123);
        assert_eq!(tts, temp.join(".voice-bridge").join("tts").join("123.wav"));
    });
}
