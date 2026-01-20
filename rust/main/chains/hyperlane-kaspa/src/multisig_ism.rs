//! Kaspa Multisig ISM implementation.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use hyperlane_core::{
    ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, InterchainSecurityModule, Metadata,
    ModuleType, MultisigIsm, H256, U256,
};
use tracing::{debug, instrument};

use crate::client::KaspaRpcClient;
use crate::provider::KaspaProvider;
use crate::trait_builder::ConnectionConf;

/// Kaspa Multisig ISM implementation.
///
/// This implements both `InterchainSecurityModule` and `MultisigIsm` traits,
/// delegating to igra-service for actual verification.
#[derive(Clone)]
pub struct KaspaMultisigIsm {
    /// Hyperlane domain
    domain: HyperlaneDomain,
    /// ISM address (group_id)
    address: H256,
    /// Kaspa provider
    provider: KaspaProvider,
    /// RPC client
    client: Arc<KaspaRpcClient>,
}

impl KaspaMultisigIsm {
    /// Create a new Kaspa Multisig ISM.
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

impl Debug for KaspaMultisigIsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaspaMultisigIsm")
            .field("domain", &self.domain)
            .field("address", &self.address)
            .finish()
    }
}

impl HyperlaneContract for KaspaMultisigIsm {
    fn address(&self) -> H256 {
        self.address
    }
}

impl HyperlaneChain for KaspaMultisigIsm {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.provider.clone())
    }
}

#[async_trait]
impl InterchainSecurityModule for KaspaMultisigIsm {
    /// Get the module type.
    ///
    /// Returns the ISM type configured in igra-service.
    #[instrument(skip(self))]
    async fn module_type(&self) -> ChainResult<ModuleType> {
        let module_type_str = self.client.get_ism_module_type().await?;

        let module_type = match module_type_str.to_lowercase().as_str() {
            "message_id_multisig" | "messageidmultisig" => ModuleType::MessageIdMultisig,
            "merkle_root_multisig" | "merklerootmultisig" => ModuleType::MerkleRootMultisig,
            "routing" => ModuleType::Routing,
            "aggregation" => ModuleType::Aggregation,
            "null" => ModuleType::Null,
            "legacy_multisig" | "legacymultisig" => ModuleType::LegacyMultisig,
            "ccip_read" | "ccipread" => ModuleType::CcipRead,
            _ => {
                debug!("unknown module type: {}, defaulting to MessageIdMultisig", module_type_str);
                ModuleType::MessageIdMultisig
            }
        };

        debug!("ISM module type: {:?}", module_type);
        Ok(module_type)
    }

    /// Dry run verification.
    ///
    /// Returns the estimated gas if verification would succeed.
    #[instrument(skip(self, message, metadata))]
    async fn dry_run_verify(
        &self,
        message: &HyperlaneMessage,
        metadata: &Metadata,
    ) -> ChainResult<Option<U256>> {
        let result = self.client.dry_run_verify(message, metadata).await?;
        debug!("dry run verify result: {:?}", result);
        Ok(result)
    }
}

#[async_trait]
impl MultisigIsm for KaspaMultisigIsm {
    /// Get validators and threshold for a message.
    ///
    /// Queries igra-service for the configured validators and threshold.
    #[instrument(skip(self, message), fields(message_id = %message.id()))]
    async fn validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        let (validators, threshold) = self
            .client
            .get_validators_and_threshold(message)
            .await?;

        debug!(
            "validators: {:?}, threshold: {}",
            validators.len(),
            threshold
        );

        Ok((validators, threshold))
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running igra-service
}
