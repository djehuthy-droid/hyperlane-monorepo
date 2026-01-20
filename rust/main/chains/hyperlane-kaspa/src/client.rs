//! HTTP client for igra-service RPC communication.

use std::time::Duration;

use hyperlane_core::{ChainResult, HyperlaneMessage, Metadata, H256, H512, U256};
use reqwest::{header, Client};
use tracing::{debug, instrument, warn};

use crate::error::{KaspaError, KaspaResult};
use crate::trait_builder::ConnectionConf;
use crate::types::*;

/// HTTP client for communicating with igra-service.
#[derive(Clone, Debug)]
pub struct KaspaRpcClient {
    http: Client,
    base_url: String,
    token: Option<String>,
}

impl KaspaRpcClient {
    /// Create a new RPC client from connection configuration.
    pub fn new(conf: &ConnectionConf) -> KaspaResult<Self> {
        conf.validate()?;

        let http = Client::builder()
            .timeout(Duration::from_secs(conf.timeout_seconds))
            .build()
            .map_err(|e| KaspaError::RpcConnection(e.to_string()))?;

        Ok(Self {
            http,
            base_url: conf.rpc_url.to_string().trim_end_matches('/').to_string(),
            token: conf.rpc_token.clone(),
        })
    }

    /// Make a POST request to the RPC endpoint.
    #[instrument(skip(self, body), fields(path = %path))]
    async fn post<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> KaspaResult<R> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = self.http.post(&url).json(body);

        if let Some(token) = &self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = req.send().await.map_err(|e| {
            warn!("RPC request failed: {}", e);
            KaspaError::RpcRequest(e.to_string())
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(KaspaError::RpcRequest(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        resp.json().await.map_err(|e| {
            KaspaError::RpcParse(format!("failed to parse response: {}", e))
        })
    }

    /// Make a GET request to the RPC endpoint.
    #[instrument(skip(self), fields(path = %path))]
    async fn get<R: serde::de::DeserializeOwned>(&self, path: &str) -> KaspaResult<R> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = self.http.get(&url);

        if let Some(token) = &self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = req.send().await.map_err(|e| {
            warn!("RPC request failed: {}", e);
            KaspaError::RpcRequest(e.to_string())
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(KaspaError::RpcRequest(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        resp.json().await.map_err(|e| {
            KaspaError::RpcParse(format!("failed to parse response: {}", e))
        })
    }

    /// Make a JSON-RPC request.
    #[instrument(skip(self, params), fields(method = %method))]
    async fn json_rpc<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> KaspaResult<R> {
        let request = JsonRpcRequest::new(method, params);
        let response: JsonRpcResponse<R> = self.post("/rpc", &request).await?;

        if let Some(error) = response.error {
            return Err(KaspaError::RpcRequest(format!(
                "JSON-RPC error {}: {}",
                error.code, error.message
            )));
        }

        response.result.ok_or_else(|| {
            KaspaError::InvalidResponse("missing result in JSON-RPC response".to_string())
        })
    }

    // ========================================================================
    // Mailbox Methods
    // ========================================================================

    /// Get the count of processed messages.
    pub async fn get_mailbox_count(&self) -> ChainResult<u32> {
        let resp: MailboxCountResponse = self.get("/rpc/mailbox/count").await?;
        Ok(resp.count)
    }

    /// Check if a message has been delivered.
    pub async fn is_message_delivered(&self, message_id: H256) -> ChainResult<bool> {
        let path = format!("/rpc/mailbox/delivered/{}", format_h256(&message_id));
        let resp: DeliveredResponse = self.get(&path).await?;
        Ok(resp.delivered)
    }

    /// Get the default ISM address (group_id).
    pub async fn get_default_ism(&self) -> ChainResult<H256> {
        let resp: DefaultIsmResponse = self.get("/rpc/mailbox/default_ism").await?;
        parse_h256(&resp.ism).map_err(|e| KaspaError::RpcParse(e).into())
    }

    /// Process a Hyperlane message.
    #[instrument(skip(self, message, metadata))]
    pub async fn process_message(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
    ) -> ChainResult<ProcessMessageResponse> {
        debug!(
            "processing message id={} origin={} destination={}",
            format_h256(&message.id()),
            message.origin,
            message.destination
        );

        let params = serde_json::json!({
            "message": encode_message(message),
            "metadata": hex::encode(metadata.as_ref()),
        });

        let resp: ProcessMessageResponse =
            self.json_rpc("hyperlane.mailbox_process", params).await?;

        if !resp.success {
            return Err(KaspaError::TransactionFailed(
                resp.error.unwrap_or_else(|| "unknown error".to_string()),
            )
            .into());
        }

        Ok(resp)
    }

    /// Estimate costs for processing a message.
    pub async fn estimate_process_costs(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
    ) -> ChainResult<EstimateCostsResponse> {
        let request = serde_json::json!({
            "message": encode_message(message),
            "metadata": hex::encode(metadata.as_ref()),
        });

        let resp: EstimateCostsResponse =
            self.post("/rpc/mailbox/estimate_costs", &request).await?;

        Ok(resp)
    }

    // ========================================================================
    // ISM Methods
    // ========================================================================

    /// Get the ISM module type.
    pub async fn get_ism_module_type(&self) -> ChainResult<String> {
        let resp: ModuleTypeResponse = self.get("/rpc/ism/module_type").await?;
        Ok(resp.module_type)
    }

    /// Dry run ISM verification.
    pub async fn dry_run_verify(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
    ) -> ChainResult<Option<U256>> {
        let request = DryRunVerifyRequest {
            message: encode_message(message),
            metadata: hex::encode(metadata.as_ref()),
        };

        let resp: DryRunVerifyResponse = self.post("/rpc/ism/dry_run_verify", &request).await?;

        if resp.success {
            if let Some(gas) = resp.gas_estimate {
                return Ok(Some(parse_u256(&gas).map_err(|e| KaspaError::RpcParse(e))?));
            }
            Ok(Some(U256::from(100_000u64))) // Default estimate
        } else {
            Ok(None)
        }
    }

    /// Get validators and threshold for a message.
    pub async fn get_validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        let params = ValidatorsAndThresholdRequest {
            message_id: format_h256(&message.id()),
            destination_domain: message.destination,
            origin_domain: message.origin,
        };

        let resp: ValidatorsAndThresholdResponse =
            self.json_rpc("hyperlane.validators_and_threshold", params).await?;

        let validators: Result<Vec<H256>, _> = resp
            .validators
            .iter()
            .map(|v| parse_h256(v))
            .collect();

        let validators = validators.map_err(|e| KaspaError::RpcParse(e))?;

        Ok((validators, resp.threshold))
    }

    // ========================================================================
    // Indexer Methods
    // ========================================================================

    /// Get the finalized block number (DAA score).
    pub async fn get_finalized_daa_score(&self) -> ChainResult<u32> {
        let resp: FinalizedBlockResponse = self.get("/rpc/indexer/finalized_block").await?;
        Ok(resp.finalized_block)
    }

    /// Get deliveries in a DAA score range.
    pub async fn get_deliveries_in_range(
        &self,
        from: u32,
        to: u32,
    ) -> ChainResult<Vec<DeliveryRecord>> {
        let path = format!("/rpc/indexer/deliveries?from={}&to={}", from, to);
        let resp: DeliveriesResponse = self.get(&path).await?;
        Ok(resp.deliveries)
    }

    /// Get sequence tip.
    pub async fn get_sequence_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let resp: SequenceTipResponse = self.get("/rpc/indexer/sequence_tip").await?;
        Ok((resp.sequence, resp.tip))
    }

    /// Get messages in a DAA score range.
    pub async fn get_messages_in_range(
        &self,
        from: u32,
        to: u32,
    ) -> ChainResult<Vec<MessageRecord>> {
        let path = format!("/rpc/indexer/messages?from={}&to={}", from, to);
        let resp: MessagesResponse = self.get(&path).await?;
        Ok(resp.messages)
    }

    // ========================================================================
    // Chain Info Methods
    // ========================================================================

    /// Get chain info.
    pub async fn get_chain_info(&self) -> ChainResult<ChainInfoResponse> {
        let resp: ChainInfoResponse = self.get("/rpc/chain/info").await?;
        Ok(resp)
    }

    /// Get block by DAA score.
    pub async fn get_block_by_daa_score(&self, daa_score: u64) -> ChainResult<BlockInfoResponse> {
        let path = format!("/rpc/chain/block/{}", daa_score);
        let resp: BlockInfoResponse = self.get(&path).await?;
        Ok(resp)
    }

    /// Get balance for an address.
    pub async fn get_balance(&self, address: &str) -> ChainResult<U256> {
        let path = format!("/rpc/chain/balance/{}", address);
        let resp: BalanceResponse = self.get(&path).await?;
        parse_u256(&resp.balance).map_err(|e| KaspaError::RpcParse(e).into())
    }

    /// Get transaction by hash.
    pub async fn get_transaction(&self, hash: &H512) -> ChainResult<TransactionInfoResponse> {
        let path = format!("/rpc/chain/transaction/{}", hex::encode(hash.as_bytes()));
        let resp: TransactionInfoResponse = self.get(&path).await?;
        Ok(resp)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Encode a HyperlaneMessage for RPC.
fn encode_message(message: &HyperlaneMessage) -> EncodedMessage {
    EncodedMessage {
        version: message.version,
        nonce: message.nonce,
        origin: message.origin,
        sender: format_h256(&message.sender),
        destination: message.destination,
        recipient: format_h256(&message.recipient),
        body: hex::encode(&message.body),
    }
}
