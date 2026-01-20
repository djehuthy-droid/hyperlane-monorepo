//! Kaspa-specific error types.

use hyperlane_core::ChainCommunicationError;
use thiserror::Error;

/// Errors specific to Kaspa chain operations.
#[derive(Error, Debug)]
pub enum KaspaError {
    /// RPC connection error
    #[error("RPC connection error: {0}")]
    RpcConnection(String),

    /// RPC request failed
    #[error("RPC request failed: {0}")]
    RpcRequest(String),

    /// RPC response parsing error
    #[error("RPC response parse error: {0}")]
    RpcParse(String),

    /// Invalid response from igra-service
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// ISM verification failed
    #[error("ISM verification failed: {0}")]
    IsmVerificationFailed(String),

    /// Timeout waiting for response
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

impl From<KaspaError> for ChainCommunicationError {
    fn from(err: KaspaError) -> Self {
        ChainCommunicationError::from_other(err)
    }
}

/// Result type for Kaspa operations.
pub type KaspaResult<T> = Result<T, KaspaError>;
