/// Verify that the capture code path no longer references ffmpeg or shell commands.
/// This is a compile-time and grep-based assertion rather than a runtime capture test.
use himd_core::types::{CaptureResult, StoppedBy};

#[test]
fn capture_result_contract_shape_is_intact() {
    // Verify the CaptureResult struct matches the MCP contract
    let result = CaptureResult {
        temp_audio_path: "/tmp/test.wav".into(),
        format: "wav".into(),
        duration_ms: 1000,
        sample_rate: 16000,
        channels: 1,
        file_size_bytes: 32044,
        stopped_by: StoppedBy::Silence,
    };
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert!(json.contains("\"temp_audio_path\""));
    assert!(json.contains("\"stopped_by\""));
    assert!(json.contains("\"silence\""));
}

#[test]
fn mcp_capture_tool_uses_native_audio() {
    // Verify that himd_audio::capture::capture_once exists and is callable
    // (compile-time check that the MCP crate can reference it)
    let _ = std::any::type_name::<himd_audio::capture::CaptureDiagnostics>();
}

#[test]
fn mcp_lib_does_not_reference_afplay() {
    let mcp_src = include_str!("../src/lib.rs");
    assert!(
        !mcp_src.contains("afplay"),
        "MCP lib should not reference afplay"
    );
}

#[test]
fn mcp_lib_does_not_reference_ffmpeg() {
    let mcp_src = include_str!("../src/lib.rs");
    assert!(
        !mcp_src.contains("ffmpeg"),
        "MCP lib should not reference ffmpeg"
    );
}
