//! Smoke tests: verify the himd binary starts and prints expected output.

use std::process::Command;

fn himd_bin() -> Command {
    // `cargo test` sets this env var to point at the built binary directory.
    let bin = env!("CARGO_BIN_EXE_himd");
    Command::new(bin)
}

#[test]
fn help_flag_succeeds() {
    let output = himd_bin()
        .arg("--help")
        .output()
        .expect("failed to run himd");
    assert!(output.status.success(), "himd --help failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("serve-stdio"),
        "help should mention serve-stdio; got: {stdout}"
    );
    assert!(
        stdout.contains("doctor"),
        "help should mention doctor; got: {stdout}"
    );
}

#[test]
fn version_flag_succeeds() {
    let output = himd_bin()
        .arg("--version")
        .output()
        .expect("failed to run himd");
    assert!(output.status.success(), "himd --version failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output should contain package version; got: {stdout}"
    );
}

#[test]
fn doctor_runs() {
    let output = himd_bin()
        .arg("doctor")
        .output()
        .expect("failed to run himd");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Doctor may exit non-zero if checks fail (e.g. missing API key in CI)
    // — we just verify it runs and produces expected output.
    assert!(
        stdout.contains("himd doctor"),
        "doctor output should contain 'himd doctor'; got: {stdout}"
    );
    assert!(
        !stdout.contains("ffmpeg"),
        "doctor should not mention ffmpeg; got: {stdout}"
    );
    assert!(
        !stdout.contains("afplay"),
        "doctor should not mention afplay; got: {stdout}"
    );
    assert!(
        stdout.contains("input"),
        "doctor should check input device; got: {stdout}"
    );
    assert!(
        stdout.contains("output"),
        "doctor should check output device; got: {stdout}"
    );
}

#[test]
fn no_args_shows_help() {
    let output = himd_bin().output().expect("failed to run himd");
    // clap exits with non-zero when no subcommand is given
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage") || stderr.contains("serve-stdio"),
        "no-args stderr should show usage; got: {stderr}"
    );
}

#[test]
fn capture_in_help() {
    let output = himd_bin()
        .arg("--help")
        .output()
        .expect("failed to run himd");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("capture"),
        "help should mention capture; got: {stdout}"
    );
    assert!(
        stdout.contains("analyze"),
        "help should mention analyze; got: {stdout}"
    );
    assert!(
        stdout.contains("say"),
        "help should mention say; got: {stdout}"
    );
}
