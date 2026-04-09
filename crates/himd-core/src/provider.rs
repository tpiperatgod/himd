//! Qwen Omni audio understanding provider.
//!
//! Uses DashScope's OpenAI-compatible endpoint to send audio + text,
//! receiving structured JSON with transcript, emotion, tone, etc.

use crate::errors::HimdError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ---------------------------------------------------------------------------
// Constants / prompts (copied exactly from the Node.js implementation)
// ---------------------------------------------------------------------------

const PROVIDER_NAME: &str = "qwen-omni";

const SYSTEM_PROMPT: &str = "你是一个专业的音频理解助手。你会收到一段音频录音。请仔细分析并输出一个 JSON 对象。

要求：
1. 首先准确转录音频中的语音内容。如果没有语音，transcript 设为空字符串。
2. 分析说话人的情绪、意图和语气。
3. 如果无法可靠判断，使用 \"unknown\" 而非猜测。
4. 对于短音频(<2秒)、嘈杂音频或非语音音频，仍返回有效结构。
5. 只输出 JSON，不要添加 markdown 围栏或其他格式。

输出格式：
{\"transcript\":\"...\",\"summary\":\"...\",\"intent\":\"...\",\"emotion\":{\"primary\":\"...\",\"confidence\":0.0},\"tone\":[\"...\"],\"key_points\":[\"...\"],\"non_verbal_signals\":[\"...\"],\"language\":\"...\"}";

const USER_PROMPT: &str = "请分析这段音频，输出结构化的 JSON 结果。";

// ---------------------------------------------------------------------------
// Internal result type
// ---------------------------------------------------------------------------

/// Internal result from audio understanding (richer than the MCP response type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioUnderstandingResult {
    pub transcript: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotion: Option<EmotionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_points: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_verbal_signals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
    pub warnings: Vec<String>,
}

/// Emotion sub-result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResult {
    pub primary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------

fn api_key() -> Result<String, HimdError> {
    std::env::var("DASHSCOPE_API_KEY")
        .map_err(|_| HimdError::Config("DASHSCOPE_API_KEY environment variable is not set.".into()))
}

fn base_url() -> String {
    std::env::var("DASHSCOPE_BASE_URL")
        .unwrap_or_else(|_| "https://dashscope.aliyuncs.com/compatible-mode/v1".into())
}

pub fn audio_model() -> String {
    std::env::var("AUDIO_MODEL").unwrap_or_else(|_| "qwen3-omni-flash".into())
}

fn is_debug() -> bool {
    std::env::var("QWEN_OMNI_DEBUG").as_deref() == Ok("true")
}

fn debug(msg: &str) {
    if is_debug() {
        eprintln!("[qwen-omni] {msg}");
    }
}

// ---------------------------------------------------------------------------
// JSON parsing with fallback (mirrors Node.js parseJsonResponse)
// ---------------------------------------------------------------------------

/// Attempt to parse a JSON string with progressive fallback.
pub fn parse_json_response(text: &str) -> Option<serde_json::Value> {
    if text.trim().is_empty() {
        return None;
    }

    // Strip markdown fences if present
    let mut cleaned = text.trim().to_string();
    if cleaned.starts_with("```") {
        cleaned = cleaned
            .strip_prefix("```json\n")
            .or_else(|| cleaned.strip_prefix("```\n"))
            .or_else(|| cleaned.strip_prefix("```"))
            .unwrap_or(&cleaned)
            .to_string();
        if cleaned.ends_with("```") {
            cleaned = cleaned.trim_end_matches("```").to_string();
        }
    }
    let cleaned = cleaned.trim();

    // 1. Direct parse
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(cleaned) {
        return Some(val);
    }

    // 2. Extract JSON object from surrounding text
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            if end > start {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cleaned[start..=end]) {
                    return Some(val);
                }
            }
        }
    }

    // 3. Repair: fix truncated/missing brackets
    let mut repaired = cleaned.to_string();
    let opens = repaired.matches('{').count();
    let closes = repaired.matches('}').count();
    if opens > closes {
        for _ in 0..(opens - closes) {
            repaired.push('}');
        }
    }
    // Fix truncated strings at end
    if let Some(pos) = repaired.rfind('"') {
        // Check if there's an odd number of quotes after the last potential string start
        let tail = &repaired[pos..];
        if !tail.contains('"') || tail.matches('"').count() == 1 {
            // Likely truncated string
        }
    }
    // Simpler approach: fix trailing commas before } or ]
    let repaired = regex_replace_trailing_commas(&repaired);

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&repaired) {
        return Some(val);
    }

    None
}

/// Remove trailing commas before } or ].
fn regex_replace_trailing_commas(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ',' && i + 1 < chars.len() {
            // Look ahead skipping whitespace
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' '
                || j < chars.len() && chars[j] == '\n'
                || j < chars.len() && chars[j] == '\r'
                || j < chars.len() && chars[j] == '\t'
            {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                // Skip the comma
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Production HTTP client using reqwest.
pub struct ReqwestClient {
    client: reqwest::Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SSE stream parsing
// ---------------------------------------------------------------------------

/// Parse SSE text chunks from a streaming response body, extracting
/// accumulated content from delta.content fields.
pub fn parse_sse_text(body: &str) -> String {
    let mut full_text = String::new();
    for line in body.lines() {
        let line = line.trim();
        if !line.starts_with("data: ") {
            continue;
        }
        let data = line.strip_prefix("data: ").unwrap_or("").trim();
        if data == "[DONE]" {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(content) = parsed
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
            {
                full_text.push_str(content);
            }
        }
    }
    full_text
}

// ---------------------------------------------------------------------------
// SSE streaming HTTP client trait
// ---------------------------------------------------------------------------

/// Streaming HTTP client for SSE responses.
pub trait StreamingHttpClient: Send + Sync {
    /// POST JSON and stream SSE response, collecting content.
    /// Returns `(status_code, collected_text)`.
    fn post_json_stream(
        &self,
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, String), HimdError>>;
}

impl StreamingHttpClient for ReqwestClient {
    fn post_json_stream(
        &self,
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, String), HimdError>> {
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
                .map_err(|e| HimdError::Io(format!("HTTP request failed: {e}")))?;

            let status = resp.status().as_u16();
            if status >= 400 {
                let err_body = resp.text().await.unwrap_or_default();
                return Err(HimdError::Api {
                    status,
                    message: err_body,
                });
            }

            let full_body = resp
                .text()
                .await
                .map_err(|e| HimdError::Io(format!("Failed to read response: {e}")))?;
            let text = parse_sse_text(&full_body);
            Ok((status, text))
        })
    }
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

/// Create an empty result with default values.
pub fn create_empty_result() -> AudioUnderstandingResult {
    AudioUnderstandingResult {
        transcript: String::new(),
        provider: PROVIDER_NAME.to_string(),
        model: audio_model(),
        summary: None,
        intent: None,
        emotion: None,
        tone: None,
        key_points: None,
        non_verbal_signals: None,
        language: None,
        confidence: 0.0,
        raw_text: None,
        warnings: Vec::new(),
    }
}

/// Normalize parsed JSON into the standard result schema.
fn normalize_result(parsed: &serde_json::Value, raw_text: &str) -> AudioUnderstandingResult {
    let mut result = create_empty_result();

    // Required: transcript
    result.transcript = parsed
        .get("transcript")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    result.raw_text = Some(raw_text.to_string());

    // Optional enriched fields
    result.summary = parsed
        .get("summary")
        .and_then(|v| v.as_str())
        .map(String::from);
    result.intent = parsed
        .get("intent")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Emotion: { primary, confidence } or string
    if let Some(emotion_val) = parsed.get("emotion") {
        if emotion_val.is_object() {
            result.emotion = Some(EmotionResult {
                primary: emotion_val
                    .get("primary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                confidence: emotion_val.get("confidence").and_then(|v| v.as_f64()),
            });
        } else if let Some(s) = emotion_val.as_str() {
            result.emotion = Some(EmotionResult {
                primary: s.to_string(),
                confidence: None,
            });
        }
    }

    // Tone: string[]
    result.tone = parsed.get("tone").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    // Key points: string[]
    result.key_points = parsed
        .get("key_points")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    // Non-verbal signals: string[]
    result.non_verbal_signals = parsed
        .get("non_verbal_signals")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    // Language
    result.language = parsed
        .get("language")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Overall confidence
    result.confidence = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    result
}

/// Build the request body for the Qwen Omni API.
fn build_request_body(audio_data_url: &str, audio_format: &str) -> serde_json::Value {
    serde_json::json!({
        "model": audio_model(),
        "stream": true,
        "stream_options": { "include_usage": true },
        "modalities": ["text"],
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": audio_data_url,
                            "format": audio_format,
                        }
                    },
                    { "type": "text", "text": USER_PROMPT }
                ]
            }
        ]
    })
}

/// Main entry: understand audio file via Qwen Omni.
///
/// Uses the provided streaming HTTP client (injectable for testing).
pub async fn understand_with_client(
    file_path: &str,
    client: &dyn StreamingHttpClient,
) -> Result<AudioUnderstandingResult, HimdError> {
    let key = api_key()?;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(HimdError::FileNotFound(file_path.to_string()));
    }

    let _model = audio_model();

    // Read and base64-encode audio file
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| HimdError::Io(format!("Failed to read audio file: {e}")))?;
    let base64_data = BASE64.encode(&file_bytes);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav")
        .to_lowercase();
    let audio_data_url = format!("data:audio/{ext};base64,{base64_data}");

    debug(&format!(
        "File: {} ({}, {}KB)",
        path.file_name().unwrap_or_default().to_string_lossy(),
        ext,
        file_bytes.len() / 1024
    ));

    let url = format!("{}/chat/completions", base_url());
    let body = build_request_body(&audio_data_url, &ext);

    // Attempt request with retry on transient errors (5xx)
    let mut raw_text: Option<String> = None;
    let mut last_error: Option<HimdError> = None;

    for attempt in 0..2 {
        match client.post_json_stream(&url, &key, &body).await {
            Ok((_status, text)) => {
                raw_text = Some(text);
                break;
            }
            Err(HimdError::Api { status, message }) if status >= 500 && attempt == 0 => {
                debug(&format!("Retry after: API error ({status})"));
                last_error = Some(HimdError::Api { status, message });
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    let raw_text = match raw_text {
        Some(t) => t,
        None => return Err(last_error.unwrap()),
    };

    // Empty response
    if raw_text.trim().is_empty() {
        let mut result = create_empty_result();
        result.warnings.push("empty_response".to_string());
        return Ok(result);
    }

    // Parse JSON response
    if let Some(parsed) = parse_json_response(&raw_text) {
        return Ok(normalize_result(&parsed, &raw_text));
    }

    // JSON parse failed — use raw text as transcript
    debug("JSON parse failed, using raw text as transcript");
    let mut result = create_empty_result();
    result.transcript = raw_text.trim().to_string();
    result.raw_text = Some(raw_text);
    result.warnings.push("json_parse_failed".to_string());
    Ok(result)
}

/// Convenience wrapper using the default production HTTP client.
pub async fn understand(file_path: &str) -> Result<AudioUnderstandingResult, HimdError> {
    let client = ReqwestClient::new();
    understand_with_client(file_path, &client).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock streaming HTTP client for testing.
    struct MockStreamingClient {
        responses: std::sync::Mutex<Vec<Result<(u16, String), HimdError>>>,
    }

    impl MockStreamingClient {
        fn new(responses: Vec<Result<(u16, String), HimdError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    impl StreamingHttpClient for MockStreamingClient {
        fn post_json_stream(
            &self,
            _url: &str,
            _api_key: &str,
            _body: &serde_json::Value,
        ) -> BoxFuture<'_, Result<(u16, String), HimdError>> {
            let resp = self.responses.lock().unwrap().pop();
            Box::pin(async move {
                match resp {
                    Some(Ok((status, body))) => {
                        // Parse SSE just like ReqwestClient does
                        let text = parse_sse_text(&body);
                        Ok((status, text))
                    }
                    Some(Err(e)) => Err(e),
                    None => Err(HimdError::Io("no mock response".into())),
                }
            })
        }
    }

    fn make_sse_body_raw(json_content: &str) -> String {
        // Build SSE body where the delta content IS the JSON string.
        // The content field value needs to be a JSON string, so we serialize
        // json_content as a JSON string and embed it in the SSE payload.
        let escaped = serde_json::to_string(json_content).unwrap_or_default();
        // escaped includes surrounding quotes; strip them for embedding
        let escaped_inner = &escaped[1..escaped.len() - 1];
        format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{escaped_inner}\"}}}}]}}\n\ndata: [DONE]\n\n"
        )
    }

    #[test]
    fn parse_sse_text_extracts_content() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\ndata: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\ndata: [DONE]\n";
        assert_eq!(parse_sse_text(body), "hello world");
    }

    #[test]
    fn parse_sse_text_skips_non_data_lines() {
        let body =
            ": comment\ndata: {\"choices\":[{\"delta\":{\"content\":\"abc\"}}]}\ndata: [DONE]\n";
        assert_eq!(parse_sse_text(body), "abc");
    }

    #[test]
    fn parse_json_response_direct() {
        let input = r#"{"transcript":"hello","summary":"greeting","intent":"greet","emotion":{"primary":"happy","confidence":0.9},"tone":["warm"],"key_points":["said hello"],"non_verbal_signals":[],"language":"en","confidence":0.8}"#;
        let result = parse_json_response(input).unwrap();
        assert_eq!(result["transcript"], "hello");
    }

    #[test]
    fn parse_json_response_strips_markdown_fences() {
        let input = "```json\n{\"transcript\":\"hello\"}\n```";
        let result = parse_json_response(input).unwrap();
        assert_eq!(result["transcript"], "hello");
    }

    #[test]
    fn parse_json_response_extracts_from_surrounding_text() {
        let input = "Here is the result:\n{\"transcript\":\"hello\"}\nEnd.";
        let result = parse_json_response(input).unwrap();
        assert_eq!(result["transcript"], "hello");
    }

    #[test]
    fn parse_json_response_repairs_missing_brackets() {
        let input = "{\"transcript\":\"hello\",\"summary\":\"greeting\"";
        let result = parse_json_response(input).unwrap();
        assert_eq!(result["transcript"], "hello");
    }

    #[test]
    fn parse_json_response_returns_none_for_garbage() {
        assert!(parse_json_response("not json at all").is_none());
    }

    #[test]
    fn normalize_result_handles_full_response() {
        let parsed = serde_json::json!({
            "transcript": "你好世界",
            "summary": "问候",
            "intent": "greeting",
            "emotion": { "primary": "happy", "confidence": 0.9 },
            "tone": ["warm"],
            "key_points": ["said hello"],
            "non_verbal_signals": ["laugh"],
            "language": "zh",
            "confidence": 0.85
        });
        let result = normalize_result(&parsed, "raw");
        assert_eq!(result.transcript, "你好世界");
        assert_eq!(result.summary.unwrap(), "问候");
        assert_eq!(result.intent.unwrap(), "greeting");
        assert_eq!(result.emotion.unwrap().primary, "happy");
        assert_eq!(result.tone.unwrap(), vec!["warm"]);
        assert_eq!(result.key_points.unwrap(), vec!["said hello"]);
        assert_eq!(result.non_verbal_signals.unwrap(), vec!["laugh"]);
        assert_eq!(result.language.unwrap(), "zh");
        assert!((result.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_result_handles_emotion_as_string() {
        let parsed = serde_json::json!({
            "transcript": "",
            "emotion": "sad"
        });
        let result = normalize_result(&parsed, "raw");
        assert_eq!(result.emotion.unwrap().primary, "sad");
    }

    #[test]
    fn normalize_result_handles_defaults() {
        let parsed = serde_json::json!({"transcript": "hello"});
        let result = normalize_result(&parsed, "raw");
        assert_eq!(result.transcript, "hello");
        assert!(result.summary.is_none());
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn understand_missing_api_key() {
        std::env::remove_var("DASHSCOPE_API_KEY");
        let client = MockStreamingClient::new(vec![]);
        let result = understand_with_client("/tmp/nonexistent.wav", &client).await;
        assert!(matches!(result.unwrap_err(), HimdError::Config(_)));
    }

    #[tokio::test]
    async fn understand_missing_file() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");
        let client = MockStreamingClient::new(vec![]);
        let result = understand_with_client("/tmp/does_not_exist_xyz.wav", &client).await;
        assert!(matches!(result.unwrap_err(), HimdError::FileNotFound(_)));
        std::env::remove_var("DASHSCOPE_API_KEY");
    }

    #[tokio::test]
    async fn understand_successful_response() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        // Create a temp WAV-like file
        let dir = std::env::temp_dir().join("himd-test-provider");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        // Minimal WAV with RIFF header
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[4..8].copy_from_slice(&(36u32.to_le_bytes()));
        wav[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&file_path, &wav).unwrap();

        let json_response = serde_json::json!({
            "transcript": "测试",
            "summary": "测试内容",
            "confidence": 0.9
        })
        .to_string();
        let sse_body = make_sse_body_raw(&json_response);

        let client = MockStreamingClient::new(vec![Ok((200, sse_body))]);
        let result = understand_with_client(file_path.to_str().unwrap(), &client).await;
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let r = result.unwrap();
        assert_eq!(r.transcript, "测试");

        // Cleanup
        std::env::remove_var("DASHSCOPE_API_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn understand_retries_on_5xx() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        let dir = std::env::temp_dir().join("himd-test-retry");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&file_path, &wav).unwrap();

        let json_response = serde_json::json!({"transcript": "retry ok"}).to_string();
        let sse_body = make_sse_body_raw(&json_response);

        // First call returns 500, second succeeds
        let client = MockStreamingClient::new(vec![
            Ok((200, sse_body)), // pop order: this is the 2nd attempt
            Err(HimdError::Api {
                status: 500,
                message: "internal error".into(),
            }), // 1st attempt
        ]);
        let result = understand_with_client(file_path.to_str().unwrap(), &client).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().transcript, "retry ok");

        std::env::remove_var("DASHSCOPE_API_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn understand_no_retry_on_4xx() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        let dir = std::env::temp_dir().join("himd-test-4xx");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&file_path, &wav).unwrap();

        let client = MockStreamingClient::new(vec![Err(HimdError::Api {
            status: 401,
            message: "unauthorized".into(),
        })]);
        let result = understand_with_client(file_path.to_str().unwrap(), &client).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            HimdError::Api { status, .. } => assert_eq!(status, 401),
            other => panic!("Expected Api error, got {other:?}"),
        }

        std::env::remove_var("DASHSCOPE_API_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn understand_empty_response_returns_warnings() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        let dir = std::env::temp_dir().join("himd-test-empty");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&file_path, &wav).unwrap();

        let client = MockStreamingClient::new(vec![Ok((200, String::new()))]);
        let result = understand_with_client(file_path.to_str().unwrap(), &client).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.warnings.contains(&"empty_response".to_string()));

        std::env::remove_var("DASHSCOPE_API_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn understand_json_parse_fallback_uses_raw_text() {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        let dir = std::env::temp_dir().join("himd-test-fallback");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        let mut wav = vec![0u8; 44];
        wav[0..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&file_path, &wav).unwrap();

        // SSE body with non-JSON content
        let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"This is not JSON at all\"}}]}\ndata: [DONE]\n";
        let client = MockStreamingClient::new(vec![Ok((200, sse_body.to_string()))]);
        let result = understand_with_client(file_path.to_str().unwrap(), &client).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transcript, "This is not JSON at all");
        assert!(r.warnings.contains(&"json_parse_failed".to_string()));

        std::env::remove_var("DASHSCOPE_API_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
