/// Regression test: Windows backend source must not shell out to external commands.
#[test]
fn windows_backend_source_does_not_shell_out() {
    let src = include_str!("../src/platform/windows.rs");
    assert!(
        !src.contains("ffmpeg"),
        "windows backend must not use ffmpeg"
    );
    assert!(
        !src.contains("afplay"),
        "windows backend must not use afplay"
    );
    assert!(
        !src.contains("powershell -c"),
        "windows backend must not shell out to powershell"
    );
}

/// Regression test: Windows backend must use cpal for input and rodio for output.
#[test]
fn windows_backend_uses_cpal_and_rodio() {
    let src = include_str!("../src/platform/windows.rs");
    assert!(
        src.contains("cpal"),
        "windows backend should reference cpal for audio input"
    );
    assert!(
        src.contains("rodio"),
        "windows backend should reference rodio for audio output"
    );
}

/// Regression test: probe functions must exist (not placeholders).
#[test]
fn windows_backend_probes_are_not_placeholder() {
    let src = include_str!("../src/platform/windows.rs");
    assert!(
        !src.contains("windows backend not implemented"),
        "windows backend should not contain placeholder 'not implemented' strings"
    );
}
