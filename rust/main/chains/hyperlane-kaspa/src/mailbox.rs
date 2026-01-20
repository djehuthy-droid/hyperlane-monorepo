//! Kaspa Mailbox implementation.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, FixedPointNumber, HyperlaneChain, HyperlaneContract,
    HyperlaneDomain, HyperlaneMessage, HyperlaneProvider, Mailbox, Metadata,
    ReorgPeriod, TxCostEstimate, TxOutcome, H256, H512, U256,
};
use tracing::{debug, info, instrument};

use crate::client::KaspaRpcClient;
use crate::provider::KaspaProvider;
use crate::trait_builder::ConnectionConf;
use crate::types::parse_h256;

/// Kaspa Mailbox contract implementation.
///
/// This connects to igra-service which acts as the "Mailbox" for Kaspa.
/// The address is the group_id of the threshold signing group.
#[derive(Clone)]
pub struct KaspaMailbox {
    /// Hyperlane domain
    domain: HyperlaneDomain,
    /// Contract address (group_id)
    address: H256,
    /// Kaspa provider
    provider: KaspaProvider,
    /// RPC client
    client: Arc<KaspaRpcClient>,
}

impl KaspaMailbox {
    /// Create a new Kaspa Mailbox.
    pub fn new(
        domain: HyperlaneDomain,
        address: H256,
        conf: &ConnectionConf,
    ) -> ChainResult<Self> {
        let client = Arc::new(KaspaRpcClient::new(conf)?);
        let provider = KaspaProvider::from_client(domain.clone(), client.clone());

        Ok(Self {
            domain,
            address,
            provider,
            client,
        })
    }

    /// Create from an existing provider and client.
    pub fn from_provider(
        address: H256,
        provider: KaspaProvider,
        client: Arc<KaspaRpcClient>,
    ) -> Self {
        Self {
            domain: provider.domain().clone(),
            address,
            provider,
            client,
        }
    }
}

impl Debug for KaspaMailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaspaMailbox")
            .field("domain", &self.domain)
            .field("address", &self.address)
            .finish()
    }
}

impl HyperlaneContract for KaspaMailbox {
    fn address(&self) -> H256 {
        self.address
    }
}

impl HyperlaneChain for KaspaMailbox {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.provider.clone())
    }
}

#[async_trait]
impl Mailbox for KaspaMailbox {
    /// Get the message count (nonce).
    ///
    /// For Kaspa, this returns the count of successfully delivered messages.
    #[instrument(skip(self))]
    async fn count(&self, _reorg_period: &ReorgPeriod) -> ChainResult<u32> {
        let count = self.client.get_mailbox_count().await?;
        debug!("mailbox count: {}", count);
        Ok(count)
    }

    /// Check if a message has been delivered.
    #[instrument(skip(self), fields(message_id = %id))]
    async fn delivered(&self, id: H256) -> ChainResult<bool> {
        let delivered = self.client.is_message_delivered(id).await?;
        debug!("message {} delivered: {}", id, delivered);
        Ok(delivered)
    }

    /// Get the default ISM address.
    ///
    /// For Kaspa, this returns the group_id.
    #[instrument(skip(self))]
    async fn default_ism(&self) -> ChainResult<H256> {
        self.client.get_default_ism().await
    }

    /// Get the ISM for a specific recipient.
    ///
    /// For Kaspa, we always return the default ISM since there's no
    /// per-recipient ISM configuration.
    #[instrument(skip(self))]
    async fn recipient_ism(&self, _recipient: H256) -> ChainResult<H256> {
        // Kaspa doesn't support per-recipient ISMs, use default
        self.default_ism().await
    }

    /// Process a Hyperlane message.
    ///
    /// This is the main entry point for message delivery. It triggers
    /// the threshold signing flow in igra-service.
    #[instrument(skip(self, message, metadata), fields(
        message_id = %message.id(),
        origin = message.origin,
        destination = message.destination,
        nonce = message.nonce,
    ))]
    async fn process(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
        _tx_gas_limit: Option<U256>,
    ) -> ChainResult<TxOutcome> {
        info!(
            "processing message from {} to {}",
            message.origin, message.destination
        );

        let response = self.client.process_message(message, metadata).await?;

        // Parse transaction hash
        let tx_hash = parse_h256(&response.transaction_hash)
            .map_err(|e| ChainCommunicationError::from_other_str(&e))?;

        // Convert to H512 (zero-padded)
        let mut hash_bytes = [0u8; 64];
        hash_bytes[..32].copy_from_slice(tx_hash.as_bytes());
        let full_hash = H512::from(hash_bytes);

        // Parse gas used
        let gas_used = response
            .gas_used
            .as_ref()
            .and_then(|g| g.parse::<u64>().ok())
            .map(U256::from)
            .unwrap_or_else(|| U256::from(100_000u64));

        info!(
            "message processed: tx_id={} tx_hash={}",
            response.transaction_id, response.transaction_hash
        );

        Ok(TxOutcome {
            transaction_id: full_hash,
            executed: true,
            gas_used,
            gas_price: FixedPointNumber::from(1i64), // Kaspa has minimal fees
        })
    }

    /// Estimate costs for processing a message.
    #[instrument(skip(self, message, metadata))]
    async fn process_estimate_costs(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
    ) -> ChainResult<TxCostEstimate> {
        let estimate = self.client.estimate_process_costs(message, metadata).await?;

        let gas_limit = estimate
            .gas_limit
            .parse::<u64>()
            .map(U256::from)
            .unwrap_or_else(|_| U256::from(100_000u64));

        let gas_price = estimate
            .gas_price
            .parse::<i64>()
            .map(FixedPointNumber::from)
            .unwrap_or_else(|_| FixedPointNumber::from(1i64));

        Ok(TxCostEstimate {
            gas_limit,
            gas_price,
            l2_gas_limit: estimate
                .l2_gas_limit
                .and_then(|l| l.parse::<u64>().ok())
                .map(U256::from),
        })
    }

    /// Get calldata for processing a message.
    ///
    /// For Kaspa/igra-service, there's no traditional calldata since
    /// we use HTTP RPC instead of on-chain calls.
    #[instrument(skip(self, _message, _metadata))]
    async fn process_calldata(
        &self,
        _message: &HyperlaneMessage,
        _metadata: &Metadata,
    ) -> ChainResult<Vec<u8>> {
        // Kaspa doesn't use calldata in the traditional sense
        Ok(vec![])
    }

    /// Get calldata for checking delivery status.
    ///
    /// Not applicable for Kaspa.
    fn delivered_calldata(&self, _message_id: H256) -> ChainResult<Option<Vec<u8>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running igra-service
}
