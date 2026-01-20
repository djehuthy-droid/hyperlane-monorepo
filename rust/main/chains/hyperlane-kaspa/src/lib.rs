//! Hyperlane Kaspa Integration
//!
//! This crate provides Hyperlane protocol support for the Kaspa blockchain
//! via the igra-service threshold signing service.
//!
//! # Architecture
//!
//! Unlike traditional EVM chains where contracts handle message routing,
//! Kaspa integration uses igra-service as the "contract" layer:
//!
//! ```text
//! Hyperlane Relayer
//!       │
//!       │  HTTP/JSON-RPC
//!       ▼
//! igra-service (threshold signing)
//!       │
//!       │  gRPC
//!       ▼
//! Kaspa Node
//! ```
//!
//! # Components
//!
//! - [`KaspaProvider`] - Implements `HyperlaneProvider` for chain queries
//! - [`KaspaMailbox`] - Implements `Mailbox` for message processing
//! - [`KaspaMultisigIsm`] - Implements ISM traits for signature verification
//! - [`KaspaMessageIndexer`] - Implements `Indexer` for message retrieval
//! - [`KaspaRpcClient`] - HTTP client for igra-service communication

mod client;
mod error;
mod indexer;
mod mailbox;
mod multisig_ism;
mod provider;
mod trait_builder;
mod types;

pub use client::KaspaRpcClient;
pub use error::KaspaError;
pub use indexer::{KaspaDeliveryIndexer, KaspaMessageIndexer};
pub use mailbox::KaspaMailbox;
pub use multisig_ism::KaspaMultisigIsm;
pub use provider::KaspaProvider;
pub use trait_builder::{ConnectionConf, RetryConfig};
pub use types::*;
