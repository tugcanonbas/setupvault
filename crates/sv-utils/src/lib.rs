//! Shared helpers and error types for SetupVault.

use thiserror::Error;

/// Result type for shared helpers.
pub type UtilsResult<T> = Result<T, UtilsError>;

/// Shared error variants for cross-crate helpers.
#[derive(Debug, Error)]
pub enum UtilsError {
    /// An IO error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A serialization error occurred.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// A parsing error occurred.
    #[error("parse error: {0}")]
    Parse(String),
}

/// Basic heuristic for detecting secrets in content.
pub fn contains_potential_secret(contents: &str) -> bool {
    let lowered = contents.to_lowercase();
    let signals = [
        "api_key",
        "apikey",
        "secret",
        "token",
        "aws_access_key_id",
        "aws_secret_access_key",
        "github_token",
        "bearer ",
        "private_key",
        "-----begin",
    ];
    signals.iter().any(|signal| lowered.contains(signal))
}
