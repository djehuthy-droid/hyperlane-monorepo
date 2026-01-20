//! Connection configuration for Kaspa chain.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::KaspaError;

/// Connection configuration for Kaspa via igra-service.
///
/// This configures how the Hyperlane agent connects to the igra-service
/// RPC endpoint, which acts as the "contract" layer for Kaspa.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionConf {
    /// Base URL of igra-service RPC (e.g., "http://127.0.0.1:8088")
    pub rpc_url: Url,

    /// Optional authentication token for igra-service
    #[serde(default)]
    pub rpc_token: Option<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Kaspa network prefix (e.g., "kaspa", "kaspatest", "kaspadev")
    #[serde(default = "default_network_prefix")]
    pub network_prefix: String,

    /// Retry configuration for failed requests
    #[serde(default)]
    pub retry_config: RetryConfig,
}

fn default_timeout() -> u64 {
    30
}

fn default_network_prefix() -> String {
    "kaspa".to_string()
}

impl ConnectionConf {
    /// Create a new connection configuration.
    pub fn new(rpc_url: Url) -> Self {
        Self {
            rpc_url,
            rpc_token: None,
            timeout_seconds: default_timeout(),
            network_prefix: default_network_prefix(),
            retry_config: RetryConfig::default(),
        }
    }

    /// Set authentication token.
    pub fn with_token(mut self, token: String) -> Self {
        self.rpc_token = Some(token);
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set network prefix.
    pub fn with_network_prefix(mut self, prefix: String) -> Self {
        self.network_prefix = prefix;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), KaspaError> {
        if self.rpc_url.scheme() != "http" && self.rpc_url.scheme() != "https" {
            return Err(KaspaError::InvalidConfig(format!(
                "RPC URL must use http or https scheme, got: {}",
                self.rpc_url.scheme()
            )));
        }

        if self.timeout_seconds == 0 {
            return Err(KaspaError::InvalidConfig(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Retry configuration for RPC requests.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial backoff delay in milliseconds
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay in milliseconds
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_backoff_ms() -> u64 {
    1000
}

fn default_max_backoff_ms() -> u64 {
    10000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given retry attempt.
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = (self.initial_backoff_ms as f64)
            * self.backoff_multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_backoff_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_conf_validation() {
        let conf = ConnectionConf::new(Url::parse("http://localhost:8088").unwrap());
        assert!(conf.validate().is_ok());

        let invalid = ConnectionConf::new(Url::parse("ftp://localhost:8088").unwrap());
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_retry_delay() {
        let config = RetryConfig::default();
        assert_eq!(config.delay_for_attempt(0), 1000);
        assert_eq!(config.delay_for_attempt(1), 2000);
        assert_eq!(config.delay_for_attempt(2), 4000);
        assert_eq!(config.delay_for_attempt(3), 8000);
        assert_eq!(config.delay_for_attempt(4), 10000); // Capped at max
    }
}
