use himd_audio::capture::CaptureDiagnostics;

#[test]
fn diagnostics_reports_input_device_result() {
    let diagnostics = CaptureDiagnostics::probe();
    assert!(
        diagnostics.summary.contains("input"),
        "diagnostics summary should mention 'input': {}",
        diagnostics.summary
    );
}
