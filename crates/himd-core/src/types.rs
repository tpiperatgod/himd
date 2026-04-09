//! MCP response types that match the Node.js voice-bridge contract exactly.
//!
//! These types are frozen — any change to field names, types, or serialization
//! will break the contract snapshot tests and must be intentional.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums for constrained values
// ---------------------------------------------------------------------------

/// Why a capture stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppedBy {
    Silence,
    NoSpeech,
    Manual,
    Timeout,
}

/// Speech rate classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechRate {
    Slow,
    Normal,
    Fast,
}

/// Energy level classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnergyLevel {
    Low,
    Medium,
    High,
}

/// Pause pattern classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PausePattern {
    Short,
    Medium,
    Long,
}

// ---------------------------------------------------------------------------
// audio_capture_once response
// ---------------------------------------------------------------------------

/// Success response from `audio_capture_once`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CaptureResult {
    pub temp_audio_path: String,
    pub format: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u32,
    pub file_size_bytes: u64,
    pub stopped_by: StoppedBy,
}

// ---------------------------------------------------------------------------
// audio_transcribe response
// ---------------------------------------------------------------------------

/// Success response from `audio_transcribe`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TranscribeResult {
    pub transcript: String,
    pub source: String,
    pub audio_file: String,
    pub model: String,
}

// ---------------------------------------------------------------------------
// audio_analyze response (the audio_turn struct)
// ---------------------------------------------------------------------------

/// Acoustic analysis sub-object within `AudioTurn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AcousticAnalysis {
    pub speech_rate: SpeechRate,
    pub energy: EnergyLevel,
    pub pause_pattern: PausePattern,
}

/// Emotion sub-object within `AudioUnderstanding`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Emotion {
    pub primary: String,
    pub confidence: Option<f64>,
}

/// Enriched audio understanding from the provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AudioUnderstanding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotion: Option<Emotion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_points: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_verbal_signals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub confidence: f64,
}

/// Success response from `audio_analyze` — the `audio_turn` struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AudioTurn {
    pub transcript: String,
    pub analysis: AcousticAnalysis,
    pub analysis_confidence: f64,
    pub source: String,
    pub audio_file: String,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_understanding: Option<AudioUnderstanding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// speech_say response
// ---------------------------------------------------------------------------

/// Success response from `speech_say`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeechResult {
    pub spoken: bool,
    pub audio_file: String,
    pub model: String,
    pub voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    pub optimize_instructions: bool,
    pub text_length: usize,
}

/// Error response from `speech_say`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeechError {
    pub spoken: bool,
    pub error: String,
}

// ---------------------------------------------------------------------------
// speech_set_profile response
// ---------------------------------------------------------------------------

/// Success response from `speech_set_profile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VoiceProfileResult {
    pub profile: VoiceProfile,
}

/// A persisted voice profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VoiceProfile {
    pub voice: String,
    pub instructions: String,
    pub optimize_instructions: bool,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Generic tool error
// ---------------------------------------------------------------------------

/// Generic error response used by most MCP tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_by_serializes_to_snake_case() {
        let cases = vec![
            (StoppedBy::Silence, "\"silence\""),
            (StoppedBy::NoSpeech, "\"no_speech\""),
            (StoppedBy::Manual, "\"manual\""),
            (StoppedBy::Timeout, "\"timeout\""),
        ];
        for (val, expected) in cases {
            assert_eq!(serde_json::to_string(&val).unwrap(), expected);
        }
    }

    #[test]
    fn speech_rate_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&SpeechRate::Slow).unwrap(),
            "\"slow\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechRate::Normal).unwrap(),
            "\"normal\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechRate::Fast).unwrap(),
            "\"fast\""
        );
    }

    #[test]
    fn energy_level_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&EnergyLevel::Low).unwrap(), "\"low\"");
        assert_eq!(
            serde_json::to_string(&EnergyLevel::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&EnergyLevel::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn pause_pattern_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&PausePattern::Short).unwrap(),
            "\"short\""
        );
        assert_eq!(
            serde_json::to_string(&PausePattern::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&PausePattern::Long).unwrap(),
            "\"long\""
        );
    }
}
