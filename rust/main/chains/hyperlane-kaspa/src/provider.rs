//! Kaspa provider implementation.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use hyperlane_core::{
    BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain,
    HyperlaneProvider, TxnInfo, H256, H512, U256,
};

use crate::client::KaspaRpcClient;
use crate::trait_builder::ConnectionConf;
use crate::types::parse_h256;

/// Kaspa provider for chain interactions.
///
/// This implements `HyperlaneProvider` using igra-service as the backend.
/// Since Kaspa is a DAG-based blockchain, some concepts map differently:
/// - Block height → DAA score
/// - Block hash → Blue block hash at DAA score
#[derive(Clone)]
pub struct KaspaProvider {
    domain: HyperlaneDomain,
    client: Arc<KaspaRpcClient>,
}

impl KaspaProvider {
    /// Create a new Kaspa provider.
    pub fn new(domain: HyperlaneDomain, conf: &ConnectionConf) -> ChainResult<Self> {
        let client = Arc::new(KaspaRpcClient::new(conf)?);
        Ok(Self { domain, client })
    }

    /// Create from an existing client.
    pub fn from_client(domain: HyperlaneDomain, client: Arc<KaspaRpcClient>) -> Self {
        Self { domain, client }
    }

    /// Get the underlying RPC client.
    pub fn client(&self) -> &KaspaRpcClient {
        &self.client
    }
}

impl Debug for KaspaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaspaProvider")
            .field("domain", &self.domain)
            .finish()
    }
}

impl HyperlaneChain for KaspaProvider {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for KaspaProvider {
    /// Get block info by DAA score.
    ///
    /// Note: Kaspa uses DAA (Difficulty Adjustment Algorithm) score instead of
    /// traditional block height. The DAA score is monotonically increasing
    /// and represents the "virtual" chain position.
    async fn get_block_by_height(&self, height: u64) -> ChainResult<BlockInfo> {
        let block = self.client.get_block_by_daa_score(height).await?;

        let hash = parse_h256(&block.hash)
            .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

        Ok(BlockInfo {
            hash,
            timestamp: block.timestamp,
            number: block.daa_score,
        })
    }

    /// Get transaction info by hash.
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let tx = self.client.get_transaction(hash).await?;

        let tx_hash = parse_h256(&tx.hash)
            .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

        // Convert tx_hash (32 bytes) to H512 by zero-padding
        let mut hash_bytes = [0u8; 64];
        hash_bytes[..32].copy_from_slice(tx_hash.as_bytes());
        let full_hash = H512::from(hash_bytes);

        Ok(TxnInfo {
            hash: full_hash,
            // Kaspa doesn't have traditional gas, use a nominal value
            gas_limit: U256::from(100_000u64),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            gas_price: Some(U256::from(1u64)), // 1 sompi/mass
            nonce: 0, // Kaspa doesn't use nonces in the same way
            sender: H256::zero(), // Would need to derive from inputs
            recipient: None,
            receipt: None,
            raw_input_data: None,
        })
    }

    /// Check if an address is a "contract".
    ///
    /// For Kaspa, we always return true since igra-service acts as the contract.
    /// The address is interpreted as the group_id.
    async fn is_contract(&self, _address: &H256) -> ChainResult<bool> {
        // In Kaspa integration, the "contract" is igra-service itself
        Ok(true)
    }

    /// Get balance for an address.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        self.client.get_balance(&address).await
    }

    /// Get chain metrics.
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let chain_info = self.client.get_chain_info().await?;

        // Get a block at the current DAA score for latest_block info
        let block = self.client.get_block_by_daa_score(chain_info.virtual_daa_score).await?;
        let hash = parse_h256(&block.hash)
            .map_err(|e| hyperlane_core::ChainCommunicationError::from_other_str(&e))?;

        Ok(Some(ChainInfo {
            latest_block: BlockInfo {
                hash,
                timestamp: block.timestamp,
                number: block.daa_score,
            },
            // Gas price is nominal for Kaspa (1 sompi per mass unit)
            min_gas_price: Some(U256::from(1u64)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here, requiring a running igra-service
}
