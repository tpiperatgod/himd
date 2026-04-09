/// Audio-specific error types.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("device error: {0}")]
    Device(String),

    #[error("stream error: {0}")]
    Stream(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
