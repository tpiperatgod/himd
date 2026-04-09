//! himd-core: shared types for the himd voice companion.
//!
//! This crate holds common data structures, error types, and configuration
//! shared between the MCP server and CLI layers.

pub mod acoustic;
pub mod capture;
pub mod errors;
pub mod provider;
pub mod runtime_paths;
pub mod tts;
pub mod types;

/// Package version from Cargo.toml.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
