/// Regression test: capture and playback must delegate through platform modules.
#[test]
fn capture_and_playback_route_through_platform_modules() {
    let capture_src = include_str!("../src/capture.rs");
    let playback_src = include_str!("../src/playback.rs");

    assert!(
        capture_src.contains("platform::capture_once_blocking"),
        "capture.rs should delegate capture to platform::capture_once_blocking"
    );
    assert!(
        playback_src.contains("platform::play_file"),
        "playback.rs should delegate playback to platform::play_file"
    );
}

/// Regression test: diagnostics must delegate through platform modules.
#[test]
fn diagnostics_route_through_platform_modules() {
    let capture_src = include_str!("../src/capture.rs");
    let playback_src = include_str!("../src/playback.rs");

    assert!(
        capture_src.contains("platform::probe_input_device"),
        "capture.rs should delegate input probing to platform::probe_input_device"
    );
    assert!(
        playback_src.contains("platform::probe_output_device"),
        "playback.rs should delegate output probing to platform::probe_output_device"
    );
}
