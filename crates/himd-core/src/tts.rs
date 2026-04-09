//! TTS synthesis via DashScope Qwen TTS API + voice profile management.
//!
//! Provides:
//! - Voice profile read/write through the shared runtime paths module
//! - TTS synthesis via DashScope multimodal-generation endpoint
//! - Audio download and save through the shared runtime paths module

use crate::errors::HimdError;
use crate::runtime_paths;
use crate::types::{VoiceProfile, VoiceProfileResult};

const DEFAULT_VOICE: &str = "Cherry";

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------

fn api_key() -> Result<String, HimdError> {
    std::env::var("DASHSCOPE_API_KEY")
        .map_err(|_| HimdError::Config("DASHSCOPE_API_KEY environment variable is not set.".into()))
}

fn tts_model() -> String {
    std::env::var("TTS_MODEL").unwrap_or_else(|_| "qwen3-tts-instruct-flash".into())
}

/// Build the TTS API URL from the base URL.
fn tts_api_url() -> String {
    let base = std::env::var("DASHSCOPE_BASE_URL")
        .unwrap_or_else(|_| "https://dashscope.aliyuncs.com/compatible-mode/v1".into());
    let origin = if let Ok(url) = url::Url::parse(&base) {
        format!(
            "{}://{}",
            url.scheme(),
            url.host_str().unwrap_or("dashscope.aliyuncs.com")
        )
    } else {
        "https://dashscope.aliyuncs.com".to_string()
    };
    format!("{origin}/api/v1/services/aigc/multimodal-generation/generation")
}

// ---------------------------------------------------------------------------
// Voice profile
// ---------------------------------------------------------------------------

/// Internal profile representation for persistence.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredProfile {
    voice: String,
    instructions: String,
    optimize_instructions: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

impl Default for StoredProfile {
    fn default() -> Self {
        Self {
            voice: DEFAULT_VOICE.to_string(),
            instructions: String::new(),
            optimize_instructions: false,
            updated_at: None,
        }
    }
}

/// Read the current voice profile from disk.
pub fn read_profile() -> VoiceProfile {
    let profile_path = runtime_paths::voice_profile_path();
    let stored = std::fs::read_to_string(&profile_path)
        .ok()
        .and_then(|data| serde_json::from_str::<StoredProfile>(&data).ok())
        .unwrap_or_default();

    VoiceProfile {
        voice: stored.voice,
        instructions: stored.instructions,
        optimize_instructions: stored.optimize_instructions,
        updated_at: stored.updated_at.unwrap_or_default(),
    }
}

/// Write profile updates to disk, returning the updated profile.
pub fn write_profile(
    voice: Option<String>,
    instructions: Option<String>,
    optimize_instructions: Option<bool>,
) -> Result<VoiceProfileResult, HimdError> {
    let profile_path = runtime_paths::voice_profile_path();
    let mut current = std::fs::read_to_string(&profile_path)
        .ok()
        .and_then(|data| serde_json::from_str::<StoredProfile>(&data).ok())
        .unwrap_or_default();

    if let Some(v) = voice {
        current.voice = v;
    }
    if let Some(i) = instructions {
        current.instructions = i;
    }
    if let Some(o) = optimize_instructions {
        current.optimize_instructions = o;
    }
    current.updated_at = Some(chrono_now_iso());

    let json = serde_json::to_string_pretty(&current)
        .map_err(|e| HimdError::Io(format!("Failed to serialize profile: {e}")))?;
    if let Some(parent) = profile_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| HimdError::Io(format!("Failed to create profile directory: {e}")))?;
    }
    std::fs::write(&profile_path, json)
        .map_err(|e| HimdError::Io(format!("Failed to write profile: {e}")))?;

    Ok(VoiceProfileResult {
        profile: VoiceProfile {
            voice: current.voice,
            instructions: current.instructions,
            optimize_instructions: current.optimize_instructions,
            updated_at: current.updated_at.unwrap_or_default(),
        },
    })
}

/// Simple ISO timestamp without pulling in chrono.
fn chrono_now_iso() -> String {
    // Use SystemTime for a basic ISO-ish timestamp
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Just use a simple format — not perfect ISO but functional
    let secs = duration.as_secs();
    format!("{}Z", humantime_timestamp(secs))
}

/// Convert unix timestamp to an ISO-like string.
fn humantime_timestamp(secs: u64) -> String {
    // Very simplified: just return seconds since epoch wrapped in a date-like format
    // For a proper implementation we'd use chrono, but let's keep deps minimal
    let days_since_epoch = secs / 86400;
    // Approximate year/month/day from days since epoch
    let (year, month, day) = days_to_ymd(days_since_epoch);
    let time_of_day_secs = secs % 86400;
    let hours = time_of_day_secs / 3600;
    let minutes = (time_of_day_secs % 3600) / 60;
    let seconds = time_of_day_secs % 60;
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.000")
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_year(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap_year(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

// ---------------------------------------------------------------------------
// TTS HTTP client trait (for testability)
// ---------------------------------------------------------------------------

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// HTTP abstraction for TTS requests, allowing mock injection in tests.
pub trait TtsHttpClient: Send + Sync {
    /// POST JSON to the TTS endpoint, return parsed JSON response.
    fn post_tts(
        &self,
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, serde_json::Value), HimdError>>;

    /// Download bytes from a URL.
    fn download(&self, url: &str) -> BoxFuture<'_, Result<Vec<u8>, HimdError>>;
}

/// Production TTS HTTP client using reqwest.
struct ReqwestTtsClient {
    client: reqwest::Client,
}

impl TtsHttpClient for ReqwestTtsClient {
    fn post_tts(
        &self,
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, serde_json::Value), HimdError>> {
        let url = url.to_string();
        let api_key = api_key.to_string();
        let body = body.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let resp = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&body)
                .send()
                .await
                .map_err(|e| HimdError::Io(format!("TTS request failed: {e}")))?;
            let status = resp.status().as_u16();
            if status >= 400 {
                let err_body = resp.text().await.unwrap_or_default();
                return Err(HimdError::Api {
                    status,
                    message: err_body,
                });
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| HimdError::Io(format!("Failed to parse TTS response: {e}")))?;
            Ok((status, json))
        })
    }

    fn download(&self, url: &str) -> BoxFuture<'_, Result<Vec<u8>, HimdError>> {
        let url = url.to_string();
        let client = self.client.clone();
        Box::pin(async move {
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| HimdError::Io(format!("Failed to download TTS audio: {e}")))?;
            if !resp.status().is_success() {
                return Err(HimdError::Api {
                    status: resp.status().as_u16(),
                    message: "Failed to download audio".into(),
                });
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| HimdError::Io(format!("Failed to read audio data: {e}")))?;
            Ok(bytes.to_vec())
        })
    }
}

// ---------------------------------------------------------------------------
// TTS synthesis
// ---------------------------------------------------------------------------

/// Result from TTS synthesis.
#[derive(Debug)]
pub struct SynthResult {
    pub audio_file: String,
    pub model: String,
    pub voice: String,
    pub instructions: Option<String>,
    pub optimize_instructions: bool,
    pub text_length: usize,
}

/// Core synthesis logic using an injectable HTTP client.
pub async fn synthesize_with_client(
    text: &str,
    voice: Option<String>,
    instructions: Option<String>,
    optimize_instructions: Option<bool>,
    client: &dyn TtsHttpClient,
) -> Result<SynthResult, HimdError> {
    if text.trim().is_empty() {
        return Err(HimdError::Validation("Text is required for TTS".into()));
    }
    if text.len() > 600 {
        return Err(HimdError::Validation(format!(
            "Text too long: {} chars (max 600)",
            text.len()
        )));
    }

    let key = api_key()?;

    let profile = read_profile();
    let effective_voice = voice.unwrap_or(profile.voice);
    let effective_instructions = instructions.unwrap_or(profile.instructions);
    let effective_optimize = optimize_instructions.unwrap_or(profile.optimize_instructions);

    let model = tts_model();
    let payload = serde_json::json!({
        "model": model,
        "input": {
            "text": text,
            "voice": effective_voice,
            "instructions": effective_instructions,
            "optimize_instructions": effective_optimize,
            "language_type": "Auto"
        }
    });

    let (_status, result) = client.post_tts(&tts_api_url(), &key, &payload).await?;

    let audio_url = result
        .get("output")
        .and_then(|o| o.get("audio"))
        .and_then(|a| a.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| HimdError::Io("TTS API did not return an audio URL".into()))?;

    let audio_bytes = client.download(audio_url).await?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let audio_file = runtime_paths::tts_output_path(timestamp);
    if let Some(parent) = audio_file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| HimdError::Io(format!("Failed to create TTS output directory: {e}")))?;
    }
    std::fs::write(&audio_file, &audio_bytes)
        .map_err(|e| HimdError::Io(format!("Failed to write audio file: {e}")))?;
    let audio_file = audio_file.to_string_lossy().to_string();

    let effective_instructions_opt = if effective_instructions.is_empty() {
        None
    } else {
        Some(effective_instructions)
    };

    Ok(SynthResult {
        audio_file,
        model,
        voice: effective_voice,
        instructions: effective_instructions_opt,
        optimize_instructions: effective_optimize,
        text_length: text.len(),
    })
}

/// Convenience wrapper using the default production HTTP client.
pub async fn synthesize(
    text: &str,
    voice: Option<String>,
    instructions: Option<String>,
    optimize_instructions: Option<bool>,
) -> Result<SynthResult, HimdError> {
    let client = ReqwestTtsClient {
        client: reqwest::Client::new(),
    };
    synthesize_with_client(text, voice, instructions, optimize_instructions, &client).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_profile_returns_default_when_no_file() {
        let temp = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();
        let _ = std::fs::remove_file(runtime_paths::voice_profile_path());
        let profile = read_profile();
        assert_eq!(profile.voice, "Cherry");
        assert_eq!(profile.instructions, "");
        assert!(!profile.optimize_instructions);
        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn write_and_read_profile() {
        // Use a unique test path to avoid parallel test interference
        let test_profile_path = format!("/tmp/himd-test-profile-{}.json", std::process::id());
        let _ = std::fs::remove_file(&test_profile_path);

        // Write initial
        let initial = StoredProfile {
            voice: "Cherry".to_string(),
            instructions: String::new(),
            optimize_instructions: false,
            updated_at: None,
        };
        std::fs::write(&test_profile_path, serde_json::to_string(&initial).unwrap()).unwrap();

        // Simulate write_profile logic with test path
        let mut stored: StoredProfile =
            serde_json::from_str(&std::fs::read_to_string(&test_profile_path).unwrap()).unwrap();
        stored.voice = "TestVoice".to_string();
        stored.instructions = "Speak clearly".to_string();
        stored.optimize_instructions = true;
        stored.updated_at = Some("2025-01-01T00:00:00.000Z".to_string());
        std::fs::write(
            &test_profile_path,
            serde_json::to_string_pretty(&stored).unwrap(),
        )
        .unwrap();

        // Read back
        let read_back: StoredProfile =
            serde_json::from_str(&std::fs::read_to_string(&test_profile_path).unwrap()).unwrap();
        assert_eq!(read_back.voice, "TestVoice");
        assert_eq!(read_back.instructions, "Speak clearly");
        assert!(read_back.optimize_instructions);

        let _ = std::fs::remove_file(&test_profile_path);
    }

    #[test]
    fn write_profile_partial_update() {
        let temp = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();
        // Write initial
        write_profile(Some("Voice1".to_string()), None, None).unwrap();
        // Partial update
        write_profile(None, Some("New instructions".to_string()), None).unwrap();
        let profile = read_profile();
        assert_eq!(profile.voice, "Voice1"); // unchanged
        assert_eq!(profile.instructions, "New instructions"); // updated

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn tts_model_default() {
        std::env::remove_var("TTS_MODEL");
        assert_eq!(tts_model(), "qwen3-tts-instruct-flash");
    }

    #[test]
    fn tts_api_url_uses_base_url_origin() {
        std::env::set_var("DASHSCOPE_BASE_URL", "https://custom.example.com/v1");
        let url = tts_api_url();
        assert!(url.starts_with("https://custom.example.com/"));
        assert!(url.contains("/api/v1/services/aigc/multimodal-generation/generation"));
        std::env::remove_var("DASHSCOPE_BASE_URL");
    }

    #[test]
    fn synthesize_empty_text_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(synthesize("", None, None, None));
        assert!(matches!(result.unwrap_err(), HimdError::Validation(_)));
    }

    #[test]
    fn synthesize_too_long_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let long_text = "a".repeat(601);
        let result = rt.block_on(synthesize(&long_text, None, None, None));
        match result.unwrap_err() {
            HimdError::Validation(msg) => assert!(msg.contains("601 chars")),
            other => panic!("Expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn humantime_timestamp_basic() {
        // 2025-01-01 00:00:00 UTC = 1735689600
        let ts = humantime_timestamp(1735689600);
        assert!(ts.starts_with("2025-01-01T"));
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }
}
