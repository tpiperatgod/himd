use himd_audio::{capture, playback};

#[test]
fn audio_crate_exports_capture_and_playback_modules() {
    let _ = std::any::type_name::<capture::CaptureDiagnostics>();
    let _ = std::any::type_name::<playback::PlaybackDiagnostics>();
}
