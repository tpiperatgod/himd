use himd_core::errors::HimdError;
use himd_core::tts::{synthesize_with_client, TtsHttpClient};
use std::sync::{Mutex, OnceLock};

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_runtime_dir<T>(f: impl FnOnce() -> T) -> T {
    let _guard = env_lock().lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    let result = f();
    std::env::set_current_dir(original).unwrap();
    result
}

struct FakeTtsClient {
    synth_response: serde_json::Value,
    download_body: Vec<u8>,
}

impl TtsHttpClient for FakeTtsClient {
    fn post_tts(
        &self,
        _url: &str,
        _api_key: &str,
        _body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, serde_json::Value), HimdError>> {
        let resp = self.synth_response.clone();
        Box::pin(async move { Ok((200, resp)) })
    }

    fn download(&self, _url: &str) -> BoxFuture<'_, Result<Vec<u8>, HimdError>> {
        let body = self.download_body.clone();
        Box::pin(async move { Ok(body) })
    }
}

#[test]
fn synthesize_uses_tts_model_env_override() {
    with_test_runtime_dir(|| {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");
        std::env::set_var("TTS_MODEL", "qwen3-tts-instruct-flash");

        let client = FakeTtsClient {
            synth_response: serde_json::json!({
                "output": {
                    "audio": {
                        "url": "https://example.com/audio.wav"
                    }
                }
            }),
            download_body: b"RIFF....WAVEfmt ".to_vec(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(synthesize_with_client(
                "Hello world",
                None,
                None,
                None,
                &client,
            ))
            .unwrap();
        assert_eq!(result.model, "qwen3-tts-instruct-flash");

        std::env::remove_var("TTS_MODEL");
        std::env::remove_var("DASHSCOPE_API_KEY");
    });
}

#[test]
fn synthesize_downloads_audio_file_without_playing_it() {
    with_test_runtime_dir(|| {
        std::env::set_var("DASHSCOPE_API_KEY", "test-key");

        let audio_bytes = vec![0u8; 100]; // fake audio
        let client = FakeTtsClient {
            synth_response: serde_json::json!({
                "output": {
                    "audio": {
                        "url": "https://example.com/audio.wav"
                    }
                }
            }),
            download_body: audio_bytes.clone(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(synthesize_with_client(
                "Test speech",
                None,
                None,
                None,
                &client,
            ))
            .unwrap();

        assert!(
            std::path::Path::new(&result.audio_file).exists(),
            "audio file should exist: {}",
            result.audio_file
        );
        let contents = std::fs::read(&result.audio_file).unwrap();
        assert_eq!(contents, audio_bytes);

        let _ = std::fs::remove_file(&result.audio_file);
        std::env::remove_var("DASHSCOPE_API_KEY");
    });
}
