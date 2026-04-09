use std::process::Command;

#[test]
fn doctor_json_reports_runtime_base_dir_and_failure_codes() {
    let temp = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himd"))
        .args(["doctor", "--json"])
        .current_dir(temp.path())
        .env_remove("DASHSCOPE_API_KEY")
        .output()
        .expect("failed to run himd doctor --json");

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to parse doctor JSON: {e}\nstdout: {stdout}\nstderr: {stderr}");
    });

    // Runtime state dir should end with .voice-bridge
    let state_dir = report["runtime_state"]["state_dir"].as_str().unwrap();
    assert!(
        state_dir.ends_with(".voice-bridge"),
        "expected state_dir to end with .voice-bridge, got: {state_dir}"
    );

    // Should report dashscope_api_key_missing
    let failure_codes = report["readiness"]["failure_codes"]
        .as_array()
        .expect("failure_codes should be an array");
    assert!(
        failure_codes
            .iter()
            .any(|code| code == "dashscope_api_key_missing"),
        "expected dashscope_api_key_missing in failure_codes, got: {failure_codes:?}"
    );

    // Should not pass readiness
    assert_eq!(report["readiness"]["pass"], false);
}

#[test]
fn doctor_json_binary_metadata() {
    let output = Command::new(env!("CARGO_BIN_EXE_himd"))
        .args(["doctor", "--json"])
        .env_remove("DASHSCOPE_API_KEY")
        .output()
        .expect("failed to run himd doctor --json");

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    // Binary section should have expected fields
    assert!(report["binary"]["version"].is_string());
    assert!(report["binary"]["platform"].is_string());
    assert!(report["binary"]["architecture"].is_string());
    assert!(report["binary"]["executable_path"].is_string());
}

#[test]
fn doctor_json_state_dir_auto_created() {
    let temp = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himd"))
        .args(["doctor", "--json"])
        .current_dir(temp.path())
        .env_remove("DASHSCOPE_API_KEY")
        .output()
        .expect("failed to run himd doctor --json");

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to parse doctor JSON: {e}\nstdout: {stdout}\nstderr: {stderr}");
    });

    // State dir should be reported as writable (auto-created)
    assert_eq!(report["runtime_state"]["writable"], true);

    // The .voice-bridge directory should actually exist on disk now
    let voice_bridge_dir = temp.path().join(".voice-bridge");
    assert!(
        voice_bridge_dir.exists(),
        ".voice-bridge dir should have been auto-created"
    );

    // runtime_state_dir_not_writable should NOT be in failure codes
    let failure_codes = report["readiness"]["failure_codes"]
        .as_array()
        .expect("failure_codes should be an array");
    assert!(
        !failure_codes.iter().any(|code| code == "runtime_state_dir_not_writable"),
        "runtime_state_dir_not_writable should not appear when dir can be created; got: {failure_codes:?}"
    );
}
