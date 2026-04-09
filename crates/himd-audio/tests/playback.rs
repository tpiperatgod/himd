use himd_audio::playback::PlaybackDiagnostics;

#[test]
fn diagnostics_reports_output_device_result() {
    let diagnostics = PlaybackDiagnostics::probe();
    assert!(
        diagnostics.summary.contains("output"),
        "diagnostics summary should mention 'output': {}",
        diagnostics.summary
    );
}
