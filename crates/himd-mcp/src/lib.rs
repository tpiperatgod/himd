//! himd-mcp: MCP server and tool registrations for the himd voice companion.
//!
//! This crate defines the MCP server struct (`HimdServer`) and implements
//! five voice-bridge tools: audio_capture_once, audio_analyze, audio_transcribe,
//! speech_say, and speech_set_profile. Audio capture/playback is handled by
//! `himd-audio` (cross-platform, native via cpal + rodio).

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ToolsCapability};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Tool parameter types
// ---------------------------------------------------------------------------

/// Parameters for `audio_capture_once` — record from the microphone.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CaptureParams {
    /// Maximum recording duration in seconds (default: 10).
    pub max_duration_sec: Option<f64>,
}

/// Parameters for `audio_analyze` — run audio understanding on a file.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnalyzeParams {
    /// Path to the audio file to analyze.
    pub file_path: String,
}

/// Parameters for `audio_transcribe` — transcribe an audio file.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TranscribeParams {
    /// Path to the audio file to transcribe.
    pub file_path: String,
}

/// Parameters for `speech_say` — synthesize and play text.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SayParams {
    /// Text to speak (max 600 chars).
    pub text: String,
    /// Voice name for TTS.
    pub voice: Option<String>,
    /// Natural-language speaking instructions.
    pub instructions: Option<String>,
    /// Whether to optimize the speaking instructions.
    pub optimize_instructions: Option<bool>,
}

/// Parameters for `speech_set_profile` — persist voice defaults.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetProfileParams {
    /// Default voice name.
    pub voice: Option<String>,
    /// Default speaking instructions.
    pub instructions: Option<String>,
    /// Whether to optimize the speaking instructions.
    pub optimize_instructions: Option<bool>,
}

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

/// The himd MCP server — holds the tool router and will eventually hold
/// runtime state (audio device handles, TTS client, etc.).
#[derive(Clone)]
pub struct HimdServer {
    #[allow(dead_code)] // read by macro-generated #[tool_handler] code
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HimdServer {
    /// Capture audio from the local microphone with VAD-based auto-stop.
    #[tool(
        name = "audio_capture_once",
        description = "Record audio from the default microphone. Returns the path to the captured WAV file."
    )]
    async fn audio_capture_once(
        &self,
        params: Parameters<CaptureParams>,
    ) -> Result<String, String> {
        let result = himd_audio::capture::capture_once(params.0.max_duration_sec)
            .await
            .map_err(|e| {
                serde_json::json!({
                    "error": e.to_string()
                })
                .to_string()
            })?;

        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }

    /// Run audio understanding (transcript + emotion/intent/tone/summary).
    #[tool(
        name = "audio_analyze",
        description = "Analyze an audio file for content and vocal signals. Returns a unified audio_turn struct."
    )]
    async fn audio_analyze(&self, params: Parameters<AnalyzeParams>) -> Result<String, String> {
        let path = &params.0.file_path;
        if !std::path::Path::new(path).exists() {
            return Err(serde_json::json!({
                "error": format!("File not found: {path}"),
                "file_path": path
            })
            .to_string());
        }

        // Run Qwen Omni audio understanding
        let provider_result = himd_core::provider::understand(path).await.map_err(|e| {
            serde_json::json!({
                "error": e.to_string(),
                "file_path": path
            })
            .to_string()
        })?;

        // Merge with local acoustic analysis
        let turn = himd_core::acoustic::build_audio_turn(&provider_result, path);
        serde_json::to_string_pretty(&turn).map_err(|e| e.to_string())
    }

    /// Transcribe an audio file.
    #[tool(
        name = "audio_transcribe",
        description = "Transcribe an audio file to text."
    )]
    async fn audio_transcribe(
        &self,
        params: Parameters<TranscribeParams>,
    ) -> Result<String, String> {
        let path = &params.0.file_path;
        if !std::path::Path::new(path).exists() {
            return Err(serde_json::json!({
                "error": format!("File not found: {path}"),
                "file_path": path
            })
            .to_string());
        }

        let provider_result = himd_core::provider::understand(path).await.map_err(|e| {
            serde_json::json!({
                "error": e.to_string(),
                "file_path": path
            })
            .to_string()
        })?;

        let result = himd_core::types::TranscribeResult {
            transcript: provider_result.transcript,
            source: "file".to_string(),
            audio_file: path.clone(),
            model: provider_result.model,
        };
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }

    /// Synthesize text to speech and play it.
    #[tool(
        name = "speech_say",
        description = "Synthesize the given text to speech and play it through the default audio output."
    )]
    async fn speech_say(&self, params: Parameters<SayParams>) -> Result<String, String> {
        let p = params.0;
        let synth = match himd_core::tts::synthesize(
            &p.text,
            p.voice.clone(),
            p.instructions.clone(),
            p.optimize_instructions,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                let err = himd_core::types::SpeechError {
                    spoken: false,
                    error: e.to_string(),
                };
                return serde_json::to_string_pretty(&err).map_err(|e| e.to_string());
            }
        };

        // Play via native audio (rodio)
        let audio_file_path = synth.audio_file.clone();
        let spoken = tokio::task::spawn_blocking(move || {
            himd_audio::playback::play_file(std::path::Path::new(&audio_file_path)).is_ok()
        })
        .await
        .unwrap_or(false);

        let result = himd_core::types::SpeechResult {
            spoken,
            audio_file: synth.audio_file,
            model: synth.model,
            voice: synth.voice,
            instructions: synth.instructions,
            optimize_instructions: synth.optimize_instructions,
            text_length: synth.text_length,
        };

        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }

    /// Persist default voice and speaking instructions.
    #[tool(
        name = "speech_set_profile",
        description = "Set the default voice profile (voice name and speaking instructions) for future TTS calls."
    )]
    async fn speech_set_profile(
        &self,
        params: Parameters<SetProfileParams>,
    ) -> Result<String, String> {
        let result = himd_core::tts::write_profile(
            params.0.voice.clone(),
            params.0.instructions.clone(),
            params.0.optimize_instructions,
        )
        .map_err(|e| {
            serde_json::json!({
                "error": e.to_string()
            })
            .to_string()
        })?;

        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }
}

impl HimdServer {
    /// Create a new server instance with all tools registered.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for HimdServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for HimdServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            server_info: Implementation {
                name: "himd".to_string(),
                version: himd_core::version().to_string(),
                ..Default::default()
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio. Blocks until the client disconnects.
pub async fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::transport::io::stdio;
    use rmcp::ServiceExt;

    let server = HimdServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The 5 tool names that MCP clients must be able to discover.
    const EXPECTED_TOOLS: &[&str] = &[
        "audio_capture_once",
        "audio_analyze",
        "audio_transcribe",
        "speech_say",
        "speech_set_profile",
    ];

    #[test]
    fn tool_router_registers_all_tools() {
        let router = HimdServer::tool_router();
        let tools = router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        for expected in EXPECTED_TOOLS {
            assert!(
                names.contains(expected),
                "missing tool: {expected}; registered: {names:?}"
            );
        }
    }

    #[test]
    fn tool_count_is_exactly_five() {
        let router = HimdServer::tool_router();
        assert_eq!(router.list_all().len(), 5);
    }

    #[test]
    fn server_info_name() {
        let server = HimdServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "himd");
    }
}
