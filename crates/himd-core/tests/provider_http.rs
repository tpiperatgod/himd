use himd_core::errors::HimdError;
use himd_core::provider::{understand_with_client, StreamingHttpClient};

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

struct FakeStreamingClient {
    status: u16,
    body: String,
}

impl StreamingHttpClient for FakeStreamingClient {
    fn post_json_stream(
        &self,
        _url: &str,
        _api_key: &str,
        _body: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(u16, String), HimdError>> {
        let status = self.status;
        let body = self.body.clone();
        Box::pin(async move {
            if status >= 400 {
                Err(HimdError::Api {
                    status,
                    message: body,
                })
            } else {
                // Return the body directly — caller already parsed SSE
                Ok((status, body))
            }
        })
    }
}

fn make_test_wav() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.wav");
    let mut wav = vec![0u8; 44];
    wav[0..4].copy_from_slice(b"RIFF");
    wav[4..8].copy_from_slice(&36u32.to_le_bytes());
    wav[8..12].copy_from_slice(b"WAVE");
    std::fs::write(&path, &wav).unwrap();
    (dir, path)
}

#[tokio::test]
async fn understand_uses_audio_model_env_override() {
    std::env::set_var("DASHSCOPE_API_KEY", "test-key");
    std::env::set_var("AUDIO_MODEL", "qwen3-omni-flash");
    let (_dir, path) = make_test_wav();

    let json = r#"{"transcript":"hello","confidence":0.9}"#;
    let client = FakeStreamingClient {
        status: 200,
        body: json.to_string(),
    };
    let result = understand_with_client(path.to_str().unwrap(), &client)
        .await
        .unwrap();
    assert_eq!(result.model, "qwen3-omni-flash");

    std::env::remove_var("AUDIO_MODEL");
    std::env::remove_var("DASHSCOPE_API_KEY");
}

#[tokio::test]
async fn understand_returns_json_parse_warning_when_body_is_not_json() {
    std::env::set_var("DASHSCOPE_API_KEY", "test-key");
    let (_dir, path) = make_test_wav();

    let client = FakeStreamingClient {
        status: 200,
        body: "This is plain text, not JSON".to_string(),
    };
    let result = understand_with_client(path.to_str().unwrap(), &client)
        .await
        .unwrap();
    assert!(
        result.warnings.contains(&"json_parse_failed".to_string()),
        "Expected json_parse_failed warning, got: {:?}",
        result.warnings
    );

    std::env::remove_var("DASHSCOPE_API_KEY");
}
