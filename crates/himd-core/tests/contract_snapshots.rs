//! Contract snapshot tests — freeze the exact JSON shapes the MCP tools return.
//!
//! If a field is renamed, removed, or its type changes, these tests fail.
//! This protects the `/hi` plugin from silent contract breakage.

use himd_core::types::*;

// ---------------------------------------------------------------------------
// audio_capture_once
// ---------------------------------------------------------------------------

#[test]
fn capture_result_serializes() {
    let result = CaptureResult {
        temp_audio_path: "/tmp/himd-voice-bridge/captures/2025-01-01/1234567890.wav".into(),
        format: "wav".into(),
        duration_ms: 3500,
        sample_rate: 16000,
        channels: 1,
        file_size_bytes: 112000,
        stopped_by: StoppedBy::Silence,
    };
    let json = serde_json::to_value(&result).unwrap();
    let expected = serde_json::json!({
        "temp_audio_path": "/tmp/himd-voice-bridge/captures/2025-01-01/1234567890.wav",
        "format": "wav",
        "duration_ms": 3500,
        "sample_rate": 16000,
        "channels": 1,
        "file_size_bytes": 112000,
        "stopped_by": "silence"
    });
    assert_eq!(json, expected);
}

#[test]
fn capture_result_no_speech_variant() {
    let result = CaptureResult {
        stopped_by: StoppedBy::NoSpeech,
        temp_audio_path: "/tmp/x.wav".into(),
        format: "wav".into(),
        duration_ms: 8000,
        sample_rate: 16000,
        channels: 1,
        file_size_bytes: 256000,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["stopped_by"], "no_speech");
}

// ---------------------------------------------------------------------------
// audio_transcribe
// ---------------------------------------------------------------------------

#[test]
fn transcribe_result_serializes() {
    let result = TranscribeResult {
        transcript: "你好世界".into(),
        source: "file".into(),
        audio_file: "/path/to/file.wav".into(),
        model: "qwen3-omni-flash".into(),
    };
    let json = serde_json::to_value(&result).unwrap();
    let expected = serde_json::json!({
        "transcript": "你好世界",
        "source": "file",
        "audio_file": "/path/to/file.wav",
        "model": "qwen3-omni-flash"
    });
    assert_eq!(json, expected);
}

// ---------------------------------------------------------------------------
// audio_analyze (audio_turn)
// ---------------------------------------------------------------------------

#[test]
fn audio_turn_full_serializes() {
    let turn = AudioTurn {
        transcript: "你好世界".into(),
        analysis: AcousticAnalysis {
            speech_rate: SpeechRate::Normal,
            energy: EnergyLevel::Medium,
            pause_pattern: PausePattern::Medium,
        },
        analysis_confidence: 0.6,
        source: "file".into(),
        audio_file: "/path/to/file.wav".into(),
        model: "qwen3-omni-flash".into(),
        provider: "qwen-omni".into(),
        audio_understanding: Some(AudioUnderstanding {
            summary: Some("User is greeting".into()),
            intent: Some("greeting".into()),
            emotion: Some(Emotion {
                primary: "happy".into(),
                confidence: Some(0.8),
            }),
            tone: Some(vec!["friendly".into(), "warm".into()]),
            key_points: Some(vec!["greeting".into()]),
            non_verbal_signals: Some(vec!["laughter".into()]),
            language: Some("zh".into()),
            confidence: 0.85,
        }),
        warnings: Some(vec!["json_parse_failed".into()]),
    };
    let json = serde_json::to_value(&turn).unwrap();

    // Verify all fields present
    assert_eq!(json["transcript"], "你好世界");
    assert_eq!(json["analysis"]["speech_rate"], "normal");
    assert_eq!(json["analysis"]["energy"], "medium");
    assert_eq!(json["analysis"]["pause_pattern"], "medium");
    assert_eq!(json["analysis_confidence"], 0.6);
    assert_eq!(json["source"], "file");
    assert_eq!(json["model"], "qwen3-omni-flash");
    assert_eq!(json["provider"], "qwen-omni");

    // Verify nested audio_understanding
    let au = &json["audio_understanding"];
    assert_eq!(au["summary"], "User is greeting");
    assert_eq!(au["intent"], "greeting");
    assert_eq!(au["emotion"]["primary"], "happy");
    assert_eq!(au["emotion"]["confidence"], 0.8);
    assert_eq!(au["tone"], serde_json::json!(["friendly", "warm"]));
    assert_eq!(au["key_points"], serde_json::json!(["greeting"]));
    assert_eq!(au["non_verbal_signals"], serde_json::json!(["laughter"]));
    assert_eq!(au["language"], "zh");
    assert_eq!(au["confidence"], 0.85);

    assert_eq!(json["warnings"], serde_json::json!(["json_parse_failed"]));
}

#[test]
fn audio_turn_minimal_omits_optional_fields() {
    let turn = AudioTurn {
        transcript: "hello".into(),
        analysis: AcousticAnalysis {
            speech_rate: SpeechRate::Normal,
            energy: EnergyLevel::Medium,
            pause_pattern: PausePattern::Medium,
        },
        analysis_confidence: 0.5,
        source: "file".into(),
        audio_file: "/tmp/a.wav".into(),
        model: "qwen3-omni-flash".into(),
        provider: "qwen-omni".into(),
        audio_understanding: None,
        warnings: None,
    };
    let json = serde_json::to_value(&turn).unwrap();

    // Optional fields must be absent, not null
    assert!(json.get("audio_understanding").is_none());
    assert!(json.get("warnings").is_none());
}

// ---------------------------------------------------------------------------
// speech_say
// ---------------------------------------------------------------------------

#[test]
fn speech_result_serializes() {
    let result = SpeechResult {
        spoken: true,
        audio_file: "/tmp/himd-tts-1234567890.wav".into(),
        model: "qwen3-tts-instruct-flash".into(),
        voice: "Cherry".into(),
        instructions: Some("Speak warmly".into()),
        optimize_instructions: false,
        text_length: 42,
    };
    let json = serde_json::to_value(&result).unwrap();
    let expected = serde_json::json!({
        "spoken": true,
        "audio_file": "/tmp/himd-tts-1234567890.wav",
        "model": "qwen3-tts-instruct-flash",
        "voice": "Cherry",
        "instructions": "Speak warmly",
        "optimize_instructions": false,
        "text_length": 42
    });
    assert_eq!(json, expected);
}

#[test]
fn speech_result_null_instructions_omitted() {
    let result = SpeechResult {
        spoken: true,
        audio_file: "/tmp/x.wav".into(),
        model: "m".into(),
        voice: "Cherry".into(),
        instructions: None,
        optimize_instructions: false,
        text_length: 5,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("instructions").is_none());
}

#[test]
fn speech_error_serializes() {
    let err = SpeechError {
        spoken: false,
        error: "DASHSCOPE_API_KEY environment variable is not set".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    let expected = serde_json::json!({
        "spoken": false,
        "error": "DASHSCOPE_API_KEY environment variable is not set"
    });
    assert_eq!(json, expected);
}

// ---------------------------------------------------------------------------
// speech_set_profile
// ---------------------------------------------------------------------------

#[test]
fn voice_profile_result_serializes() {
    let result = VoiceProfileResult {
        profile: VoiceProfile {
            voice: "Cherry".into(),
            instructions: "Speak warmly".into(),
            optimize_instructions: false,
            updated_at: "2025-01-01T12:00:00.000Z".into(),
        },
    };
    let json = serde_json::to_value(&result).unwrap();
    let expected = serde_json::json!({
        "profile": {
            "voice": "Cherry",
            "instructions": "Speak warmly",
            "optimize_instructions": false,
            "updated_at": "2025-01-01T12:00:00.000Z"
        }
    });
    assert_eq!(json, expected);
}

// ---------------------------------------------------------------------------
// Generic tool error
// ---------------------------------------------------------------------------

#[test]
fn tool_error_with_file_path() {
    let err = ToolError {
        error: "File not found".into(),
        file_path: Some("/path/to/missing.wav".into()),
    };
    let json = serde_json::to_value(&err).unwrap();
    let expected = serde_json::json!({
        "error": "File not found",
        "file_path": "/path/to/missing.wav"
    });
    assert_eq!(json, expected);
}

#[test]
fn tool_error_without_file_path() {
    let err = ToolError {
        error: "Something went wrong".into(),
        file_path: None,
    };
    let json = serde_json::to_value(&err).unwrap();
    assert!(json.get("file_path").is_none());
    assert_eq!(json["error"], "Something went wrong");
}
