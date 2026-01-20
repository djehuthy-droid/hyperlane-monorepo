//! Kaspa indexer implementations.

use std::fmt::Debug;
use std::ops::RangeInclusive;
use std::sync::Arc;

use async_trait::async_trait;
use hyperlane_core::{
    ChainResult, HyperlaneMessage, Indexed, Indexer, LogMeta, SequenceAwareIndexer,
    H256, H512,
};
use tracing::{debug, instrument};

use crate::client::KaspaRpcClient;
use crate::trait_builder::ConnectionConf;
use crate::types::parse_h256;

/// Kaspa message indexer.
///
/// Indexes Hyperlane messages processed by igra-service.
/// Note: Since Kaspa is destination-only in this integration,
/// this primarily returns delivered (processed) messages.
#[derive(Clone)]
pub struct KaspaMessageIndexer {
    client: Arc<KaspaRpcClient>,
}

impl KaspaMessageIndexer {
    /// Create a new message indexer.
    pub fn new(conf: &ConnectionConf) -> ChainResult<Self> {
        let client = Arc::new(KaspaRpcClient::new(conf)?);
        Ok(Self { client })
    }

    /// Create from an existing client.
    pub fn from_client(client: Arc<KaspaRpcClient>) -> Self {
        Self { client }
    }
}

impl Debug for KaspaMessageIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaspaMessageIndexer").finish()
    }
}

#[async_trait]
impl Indexer<HyperlaneMessage> for KaspaMessageIndexer {
    /// Fetch messages in a DAA score range.
    ///
    /// Returns processed (delivered) messages within the range.
    #[instrument(skip(self))]
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        let from = *range.start();
        let to = *range.end();

        debug!("fetching messages in range {}..{}", from, to);

        let records = self.client.get_messages_in_range(from, to).await?;
        let mut results = Vec::with_capacity(records.len());

        for record in records {
            // Parse addresses
            let sender = parse_h256(&record.sender)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;
            let recipient = parse_h256(&record.recipient)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

            // Decode body
            let body = hex::decode(&record.body)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&format!("invalid body hex: {}", e)))?;

            // Construct message
            let message = HyperlaneMessage {
                version: 3, // Hyperlane v3
                nonce: record.nonce,
                origin: record.origin,
                sender,
                destination: record.destination,
                recipient,
                body,
            };

            // Parse transaction hash
            let tx_hash = parse_h256(&record.tx_id)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

            // Convert to H512 (zero-padded)
            let mut hash_bytes = [0u8; 64];
            hash_bytes[..32].copy_from_slice(tx_hash.as_bytes());
            let full_hash = H512::from(hash_bytes);

            // Build log metadata
            let log_meta = LogMeta {
                address: H256::zero(), // igra-service address would go here
                block_number: record.daa_score,
                block_hash: H256::zero(), // Would need blue block hash
                transaction_id: full_hash,
                transaction_index: 0,
                log_index: U256::from(record.log_index),
            };

            let indexed = Indexed::new(message);
            results.push((indexed, log_meta));
        }

        debug!("fetched {} messages", results.len());
        Ok(results)
    }

    /// Get the finalized block number (DAA score).
    #[instrument(skip(self))]
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        let finalized = self.client.get_finalized_daa_score().await?;
        debug!("finalized DAA score: {}", finalized);
        Ok(finalized)
    }
}

#[async_trait]
impl SequenceAwareIndexer<HyperlaneMessage> for KaspaMessageIndexer {
    /// Get the latest sequence count and tip.
    #[instrument(skip(self))]
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let (sequence, tip) = self.client.get_sequence_tip().await?;
        debug!("sequence: {:?}, tip: {}", sequence, tip);
        Ok((sequence, tip))
    }
}

// ============================================================================
// Delivery Indexer
// ============================================================================

/// Kaspa delivery indexer.
///
/// Indexes message delivery confirmations (message IDs that have been processed).
#[derive(Clone)]
pub struct KaspaDeliveryIndexer {
    client: Arc<KaspaRpcClient>,
}

impl KaspaDeliveryIndexer {
    /// Create a new delivery indexer.
    pub fn new(conf: &ConnectionConf) -> ChainResult<Self> {
        let client = Arc::new(KaspaRpcClient::new(conf)?);
        Ok(Self { client })
    }

    /// Create from an existing client.
    pub fn from_client(client: Arc<KaspaRpcClient>) -> Self {
        Self { client }
    }
}

impl Debug for KaspaDeliveryIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaspaDeliveryIndexer").finish()
    }
}

#[async_trait]
impl Indexer<H256> for KaspaDeliveryIndexer {
    /// Fetch delivery records in a DAA score range.
    #[instrument(skip(self))]
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<H256>, LogMeta)>> {
        let from = *range.start();
        let to = *range.end();

        debug!("fetching deliveries in range {}..{}", from, to);

        let records = self.client.get_deliveries_in_range(from, to).await?;
        let mut results = Vec::with_capacity(records.len());

        for record in records {
            // Parse message ID
            let message_id = parse_h256(&record.message_id)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

            // Parse transaction hash
            let tx_hash = parse_h256(&record.tx_id)
                .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

            // Convert to H512 (zero-padded)
            let mut hash_bytes = [0u8; 64];
            hash_bytes[..32].copy_from_slice(tx_hash.as_bytes());
            let full_hash = H512::from(hash_bytes);

            // Build log metadata
            let log_meta = LogMeta {
                address: H256::zero(),
                block_number: record.daa_score,
                block_hash: H256::zero(),
                transaction_id: full_hash,
                transaction_index: 0,
                log_index: U256::zero(),
            };

            let indexed = Indexed::new(message_id);
            results.push((indexed, log_meta));
        }

        debug!("fetched {} deliveries", results.len());
        Ok(results)
    }

    /// Get the finalized block number (DAA score).
    #[instrument(skip(self))]
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        self.client.get_finalized_daa_score().await
    }
}

#[async_trait]
impl SequenceAwareIndexer<H256> for KaspaDeliveryIndexer {
    /// Get the latest sequence count and tip for deliveries.
    #[instrument(skip(self))]
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        self.client.get_sequence_tip().await
    }
}

// Need to import U256 for log_index
use hyperlane_core::U256;

#[cfg(test)]
mod tests {
    // Integration tests would require a running igra-service
}
