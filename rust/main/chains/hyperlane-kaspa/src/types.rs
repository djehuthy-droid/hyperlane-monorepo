//! Kaspa-specific types for igra-service RPC communication.

use hyperlane_core::{H256, U256};
use serde::{Deserialize, Serialize};

// ============================================================================
// JSON-RPC Types
// ============================================================================

/// JSON-RPC request wrapper
#[derive(Clone, Debug, Serialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

impl<T> JsonRpcRequest<T> {
    pub fn new(method: &str, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC response wrapper
#[derive(Clone, Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Clone, Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// ============================================================================
// Mailbox Types
// ============================================================================

/// Request to process a Hyperlane message
#[derive(Clone, Debug, Serialize)]
pub struct ProcessMessageRequest {
    /// Encoded Hyperlane message
    pub message: EncodedMessage,
    /// ISM metadata (signatures + checkpoint)
    pub metadata: String,
}

/// Encoded Hyperlane message for RPC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedMessage {
    /// Message version
    pub version: u8,
    /// Nonce
    pub nonce: u32,
    /// Origin domain
    pub origin: u32,
    /// Sender address (hex)
    pub sender: String,
    /// Destination domain
    pub destination: u32,
    /// Recipient address (hex)
    pub recipient: String,
    /// Message body (hex)
    pub body: String,
}

/// Response from process message
#[derive(Clone, Debug, Deserialize)]
pub struct ProcessMessageResponse {
    /// Transaction ID on Kaspa
    pub transaction_id: String,
    /// Transaction hash
    pub transaction_hash: String,
    /// Gas used (in sompi)
    pub gas_used: Option<String>,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Mailbox count response
#[derive(Clone, Debug, Deserialize)]
pub struct MailboxCountResponse {
    pub count: u32,
}

/// Message delivered check response
#[derive(Clone, Debug, Deserialize)]
pub struct DeliveredResponse {
    pub delivered: bool,
}

/// Default ISM response
#[derive(Clone, Debug, Deserialize)]
pub struct DefaultIsmResponse {
    pub ism: String,
}

/// Cost estimate response
#[derive(Clone, Debug, Deserialize)]
pub struct EstimateCostsResponse {
    pub gas_limit: String,
    pub gas_price: String,
    pub l2_gas_limit: Option<String>,
}

// ============================================================================
// ISM Types
// ============================================================================

/// ISM module type response
#[derive(Clone, Debug, Deserialize)]
pub struct ModuleTypeResponse {
    pub module_type: String,
}

/// Dry run verify request
#[derive(Clone, Debug, Serialize)]
pub struct DryRunVerifyRequest {
    pub message: EncodedMessage,
    pub metadata: String,
}

/// Dry run verify response
#[derive(Clone, Debug, Deserialize)]
pub struct DryRunVerifyResponse {
    pub success: bool,
    pub gas_estimate: Option<String>,
    pub error: Option<String>,
}

/// Validators and threshold request
#[derive(Clone, Debug, Serialize)]
pub struct ValidatorsAndThresholdRequest {
    pub message_id: String,
    pub destination_domain: u32,
    pub origin_domain: u32,
}

/// Validators and threshold response
#[derive(Clone, Debug, Deserialize)]
pub struct ValidatorsAndThresholdResponse {
    pub validators: Vec<String>,
    pub threshold: u8,
}

// ============================================================================
// Indexer Types
// ============================================================================

/// Range query parameters
#[derive(Clone, Debug, Serialize)]
pub struct RangeParams {
    pub from: u32,
    pub to: u32,
}

/// Finalized block response
#[derive(Clone, Debug, Deserialize)]
pub struct FinalizedBlockResponse {
    pub finalized_block: u32,
}

/// Delivery record
#[derive(Clone, Debug, Deserialize)]
pub struct DeliveryRecord {
    pub message_id: String,
    pub tx_id: String,
    pub daa_score: u64,
    pub timestamp_nanos: u64,
}

/// Deliveries response
#[derive(Clone, Debug, Deserialize)]
pub struct DeliveriesResponse {
    pub deliveries: Vec<DeliveryRecord>,
}

/// Sequence tip response
#[derive(Clone, Debug, Deserialize)]
pub struct SequenceTipResponse {
    pub sequence: Option<u32>,
    pub tip: u32,
}

/// Message record for indexer
#[derive(Clone, Debug, Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub sender: String,
    pub recipient: String,
    pub origin: u32,
    pub destination: u32,
    pub body: String,
    pub nonce: u32,
    pub tx_id: String,
    pub daa_score: u64,
    pub log_index: u32,
}

/// Messages response
#[derive(Clone, Debug, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageRecord>,
}

// ============================================================================
// Chain Info Types
// ============================================================================

/// Chain info response
#[derive(Clone, Debug, Deserialize)]
pub struct ChainInfoResponse {
    pub virtual_daa_score: u64,
    pub past_median_time: u64,
    pub pruning_point_hash: String,
    pub network_name: String,
    pub is_synced: bool,
}

/// Block info response
#[derive(Clone, Debug, Deserialize)]
pub struct BlockInfoResponse {
    pub hash: String,
    pub daa_score: u64,
    pub timestamp: u64,
    pub blue_score: u64,
}

/// Balance response
#[derive(Clone, Debug, Deserialize)]
pub struct BalanceResponse {
    pub balance: String,
}

/// Transaction info response
#[derive(Clone, Debug, Deserialize)]
pub struct TransactionInfoResponse {
    pub tx_id: String,
    pub hash: String,
    pub block_hash: Option<String>,
    pub daa_score: Option<u64>,
    pub timestamp: Option<u64>,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
}

/// Transaction input
#[derive(Clone, Debug, Deserialize)]
pub struct TxInput {
    pub previous_outpoint_hash: String,
    pub previous_outpoint_index: u32,
    pub signature_script: String,
}

/// Transaction output
#[derive(Clone, Debug, Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub script_public_key: String,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Parse hex string to H256
pub fn parse_h256(s: &str) -> Result<H256, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(H256::from(arr))
}

/// Format H256 as hex string with 0x prefix
pub fn format_h256(h: &H256) -> String {
    format!("0x{}", hex::encode(h.as_bytes()))
}

/// Parse string to U256
pub fn parse_u256(s: &str) -> Result<U256, String> {
    // Try parsing as decimal first
    if let Ok(n) = s.parse::<u128>() {
        return Ok(U256::from(n));
    }
    // Try parsing as hex
    let s = s.strip_prefix("0x").unwrap_or(s);
    U256::from_str_radix(s, 16).map_err(|e| format!("invalid U256: {}", e))
}
