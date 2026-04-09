//! Error types for the himd voice bridge.

use std::fmt;

/// Top-level error type for himd operations.
#[derive(Debug)]
pub enum HimdError {
    /// An API key or credential is missing.
    Config(String),
    /// A file was not found or could not be read.
    FileNotFound(String),
    /// Input validation failed (e.g. file too large, wrong format).
    Validation(String),
    /// An HTTP request to a remote API failed.
    Api { status: u16, message: String },
    /// An I/O or system error occurred.
    Io(String),
    /// A required system dependency is missing.
    Dependency(String),
}

impl fmt::Display for HimdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HimdError::Config(msg) => write!(f, "{msg}"),
            HimdError::FileNotFound(path) => write!(f, "File not found: {path}"),
            HimdError::Validation(msg) => write!(f, "{msg}"),
            HimdError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            HimdError::Io(msg) => write!(f, "{msg}"),
            HimdError::Dependency(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HimdError {}
