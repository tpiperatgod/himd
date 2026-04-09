//! Structured doctor probing and rendering logic.
//!
//! Shared by CLI output modes (human, JSON). Keeps `main.rs` as a thin
//! parsing and dispatch layer.

use serde::Serialize;

// ---------------------------------------------------------------------------
// Failure codes
// ---------------------------------------------------------------------------

/// Stable failure codes consumed by the Claude plugin.
///
/// Some codes are produced by the Rust binary during `himd doctor`. Others are
/// defined here for the Claude plugin to use when it detects issues outside the
/// binary's visibility (e.g. the binary itself is missing or not executable).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum FailureCode {
    // Produced by Rust doctor:
    DashscopeApiKeyMissing,
    AudioInputMissing,
    AudioInputInitFailed,
    AudioOutputMissing,
    AudioOutputInitFailed,
    RuntimeStateDirNotWritable,
    BinaryArchMismatch,
    BinaryQuarantined,
    // Produced by Claude plugin (cannot be self-diagnosed):
    BinaryMissing,
    BinaryNotExecutable,
}

// ---------------------------------------------------------------------------
// Report data model
// ---------------------------------------------------------------------------

/// Binary metadata section.
#[derive(Debug, Serialize)]
pub struct BinaryInfo {
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub executable_path: String,
}

/// Config section.
#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    pub dashscope_api_key_set: bool,
    pub audio_model: String,
    pub tts_model: String,
    pub dashscope_base_url: Option<String>,
}

/// Audio device capability.
#[derive(Debug, Serialize)]
pub struct AudioCapability {
    pub available: bool,
    pub device_name: Option<String>,
    /// Additional detail when available but init might fail.
    pub init_ok: bool,
}

/// Runtime state section.
#[derive(Debug, Serialize)]
pub struct RuntimeState {
    pub state_dir: String,
    pub writable: bool,
}

/// Overall readiness summary.
#[derive(Debug, Serialize)]
pub struct Readiness {
    pub pass: bool,
    pub warnings: Vec<String>,
    pub failure_codes: Vec<FailureCode>,
}

/// Full doctor report — the stable machine-readable contract.
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub binary: BinaryInfo,
    pub config: ConfigInfo,
    pub audio_input: AudioCapability,
    pub audio_output: AudioCapability,
    pub runtime_state: RuntimeState,
    pub readiness: Readiness,
}

// ---------------------------------------------------------------------------
// Diagnostic logic
// ---------------------------------------------------------------------------

/// Run all diagnostics and produce a structured report.
pub fn run_diagnostics() -> DoctorReport {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let platform = std::env::consts::OS.to_string();
    let architecture = std::env::consts::ARCH.to_string();
    let executable_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let binary = BinaryInfo {
        version,
        platform,
        architecture,
        executable_path,
    };

    let dashscope_api_key_set = std::env::var("DASHSCOPE_API_KEY").is_ok();
    let audio_model = std::env::var("AUDIO_MODEL").unwrap_or_else(|_| "qwen3-omni-flash".into());
    let tts_model =
        std::env::var("TTS_MODEL").unwrap_or_else(|_| "qwen3-tts-instruct-flash".into());
    let dashscope_base_url = std::env::var("DASHSCOPE_BASE_URL").ok();

    let config = ConfigInfo {
        dashscope_api_key_set,
        audio_model,
        tts_model,
        dashscope_base_url,
    };

    let capture = himd_audio::capture::CaptureDiagnostics::probe();
    let audio_input = AudioCapability {
        available: capture.ok,
        device_name: capture.device_name,
        init_ok: capture.init_ok,
    };

    let playback = himd_audio::playback::PlaybackDiagnostics::probe();
    let audio_output = AudioCapability {
        available: playback.ok,
        device_name: playback.device_name,
        init_ok: playback.init_ok,
    };

    // Runtime state directory via shared runtime paths
    let base_dir = himd_core::runtime_paths::runtime_base_dir();
    let state_dir = base_dir.to_string_lossy().to_string();
    let writable = check_state_dir_writable(&state_dir);

    let runtime_state = RuntimeState {
        state_dir,
        writable,
    };

    // Collect failures
    let warnings = Vec::new();
    let mut failure_codes = Vec::new();

    // Binary-level checks
    if check_arch_mismatch(&binary.architecture) {
        failure_codes.push(FailureCode::BinaryArchMismatch);
    }
    if check_quarantined(&binary.executable_path) {
        failure_codes.push(FailureCode::BinaryQuarantined);
    }

    if !dashscope_api_key_set {
        failure_codes.push(FailureCode::DashscopeApiKeyMissing);
    }
    if !audio_input.available {
        failure_codes.push(FailureCode::AudioInputMissing);
    } else if !audio_input.init_ok {
        failure_codes.push(FailureCode::AudioInputInitFailed);
    }
    if !audio_output.available {
        failure_codes.push(FailureCode::AudioOutputMissing);
    } else if !audio_output.init_ok {
        failure_codes.push(FailureCode::AudioOutputInitFailed);
    }
    if !writable {
        failure_codes.push(FailureCode::RuntimeStateDirNotWritable);
    }

    let pass = failure_codes.is_empty();

    let readiness = Readiness {
        pass,
        warnings,
        failure_codes,
    };

    DoctorReport {
        binary,
        config,
        audio_input,
        audio_output,
        runtime_state,
        readiness,
    }
}

// ---------------------------------------------------------------------------
// Binary checks
// ---------------------------------------------------------------------------

/// Detect architecture mismatch between compile-time target and the running kernel.
fn check_arch_mismatch(compile_arch: &str) -> bool {
    if cfg!(target_os = "macos") {
        let native_arch = std::process::Command::new("uname")
            .arg("-m")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        if let Some(ref native) = native_arch {
            let compile = match compile_arch {
                "aarch64" => "arm64",
                "x86_64" => "x86_64",
                other => other,
            };
            return compile != native.as_str();
        }
    }
    false
}

/// Detect whether macOS has quarantined the binary (downloaded from internet).
fn check_quarantined(exe_path: &str) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    std::process::Command::new("xattr")
        .arg("-p")
        .arg("com.apple.quarantine")
        .arg(exe_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_state_dir_writable(dir: &str) -> bool {
    let path = std::path::Path::new(dir);
    // Auto-create the state directory if it doesn't exist yet.
    // This ensures new users pass doctor on first run.
    if !path.exists() && std::fs::create_dir_all(path).is_err() {
        return false;
    }
    let probe = path.join("himd-doctor-write-probe");
    match std::fs::write(&probe, b"test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Output formatters
// ---------------------------------------------------------------------------

/// Print the report as pretty-printed JSON.
pub fn print_json(report: &DoctorReport) {
    println!("{}", serde_json::to_string_pretty(report).unwrap());
}

/// Print the report in human-readable format.
pub fn print_human(report: &DoctorReport) {
    println!("himd doctor v{}", report.binary.version);
    println!();

    // Binary
    let has_arch_mismatch = report
        .readiness
        .failure_codes
        .iter()
        .any(|c| matches!(c, FailureCode::BinaryArchMismatch));
    let has_quarantine = report
        .readiness
        .failure_codes
        .iter()
        .any(|c| matches!(c, FailureCode::BinaryQuarantined));

    if has_arch_mismatch {
        println!("  [FAIL] architecture mismatch — binary was compiled for a different architecture than this machine");
    } else {
        println!(
            "  [ok] binary arch: {} on {}",
            report.binary.architecture, report.binary.platform
        );
    }

    if has_quarantine {
        println!(
            "  [FAIL] binary is quarantined by macOS — run: xattr -d com.apple.quarantine {}",
            report.binary.executable_path
        );
    }

    // API key
    if report.config.dashscope_api_key_set {
        println!("  [ok] DASHSCOPE_API_KEY is set");
    } else {
        println!("  [FAIL] DASHSCOPE_API_KEY not set — export DASHSCOPE_API_KEY=your-key");
    }

    // Models
    println!("  [ok] audio model: {}", report.config.audio_model);
    println!("  [ok] tts model: {}", report.config.tts_model);

    // Input
    if report.audio_input.available {
        let name = report
            .audio_input
            .device_name
            .as_deref()
            .unwrap_or("unknown");
        if report.audio_input.init_ok {
            println!("  [ok] default input device: {name}");
        } else {
            println!("  [FAIL] default input device: {name} (stream init failed)");
        }
    } else {
        println!("  [FAIL] no default input device found");
    }

    // Output
    if report.audio_output.available {
        let name = report
            .audio_output
            .device_name
            .as_deref()
            .unwrap_or("unknown");
        if report.audio_output.init_ok {
            println!("  [ok] default output device: {name}");
        } else {
            println!("  [FAIL] default output device: {name} (stream init failed)");
        }
    } else {
        println!("  [FAIL] no default output device found");
    }

    // Binary version
    println!("  [ok] himd binary version {}", report.binary.version);

    println!();
    if report.readiness.pass {
        println!("All checks passed. Run /hi to start.");
    } else {
        println!("Some checks failed. Fix the issues above and re-run.");
    }
}
