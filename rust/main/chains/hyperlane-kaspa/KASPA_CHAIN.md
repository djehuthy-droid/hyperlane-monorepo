# Hyperlane Kaspa Integration

This crate provides Hyperlane protocol support for the Kaspa blockchain via the igra-service threshold signing service.

## Architecture Overview

Unlike traditional EVM chains where smart contracts handle message routing, Kaspa integration uses igra-service as the "contract" layer:

```
Hyperlane Relayer
      │
      │  HTTP/JSON-RPC
      ▼
igra-service (threshold signing, 2-of-3)
      │
      │  gRPC
      ▼
Kaspa Node
```

### Key Differences from EVM Chains

| Concept | EVM | Kaspa |
|---------|-----|-------|
| Block height | Block number | DAA score |
| Contract | Solidity smart contract | igra-service RPC |
| Transaction fee | Gas (wei) | Mass (sompi) |
| Finality | N block confirmations | DAA score finality |
| Signing | EOA private key | Threshold signatures (2-of-3) |

## Files Created

| File | Description |
|------|-------------|
| `Cargo.toml` | Package configuration with dependencies |
| `src/lib.rs` | Module exports |
| `src/error.rs` | Kaspa-specific error types with `ChainCommunicationError` conversion |
| `src/types.rs` | RPC request/response types for igra-service communication |
| `src/trait_builder.rs` | `ConnectionConf` and `RetryConfig` for Kaspa connections |
| `src/client.rs` | HTTP client for igra-service RPC |
| `src/provider.rs` | `KaspaProvider` implementing `HyperlaneProvider` |
| `src/mailbox.rs` | `KaspaMailbox` implementing `Mailbox` trait |
| `src/multisig_ism.rs` | `KaspaMultisigIsm` implementing `InterchainSecurityModule` and `MultisigIsm` |
| `src/indexer.rs` | `KaspaMessageIndexer` and `KaspaDeliveryIndexer` implementing `Indexer` and `SequenceAwareIndexer` |

## Integration Points Modified

1. **hyperlane-core/src/chain.rs** - Added `Kaspa` variant to `HyperlaneDomainProtocol` enum
2. **hyperlane-base/Cargo.toml** - Added `hyperlane-kaspa` as optional dependency with `kaspa` feature
3. **hyperlane-base/src/settings/chains.rs** - Added `ChainConnectionConf::Kaspa` variant and all builder methods
4. **hyperlane-base/src/settings/parser/connection_parser.rs** - Added `build_kaspa_connection_conf` function and protocol support check
5. **hyperlane-base/src/contract_sync/cursors/mod.rs** - Added Kaspa to all `Indexable` cursor type match statements
6. **Cargo.toml (workspace)** - Added `hyperlane-kaspa` to workspace members

## Usage

### Build with Kaspa Support

```bash
cargo build -p hyperlane-base --features kaspa
```

### Configuration Example

```json
{
  "kaspa": {
    "name": "kaspa",
    "domainId": 87654321,
    "protocol": "kaspa",
    "rpcUrls": ["http://igra-service:8088"],
    "rpcToken": "optional-auth-token",
    "timeoutSeconds": 30,
    "networkPrefix": "kaspa",
    "addresses": {
      "mailbox": "0x<group_id_as_h256>",
      "interchainSecurityModule": "0x<group_id_as_h256>"
    }
  }
}
```

### Retry Configuration (Optional)

```json
{
  "kaspa": {
    "retry": {
      "maxRetries": 3,
      "initialBackoffMs": 1000,
      "maxBackoffMs": 10000,
      "backoffMultiplier": 2.0
    }
  }
}
```

## Implemented Traits

### HyperlaneProvider (`KaspaProvider`)
- `get_block_by_height()` - Get block info by DAA score
- `get_txn_by_hash()` - Get transaction info
- `is_contract()` - Always returns true (igra-service is the "contract")
- `get_balance()` - Get address balance
- `get_chain_metrics()` - Get chain info (latest block, gas price)

### Mailbox (`KaspaMailbox`)
- `count()` - Get processed message count
- `delivered()` - Check if message has been delivered
- `default_ism()` - Get default ISM (group_id)
- `recipient_ism()` - Get recipient ISM (returns default)
- `process()` - Process a Hyperlane message (triggers threshold signing)
- `process_estimate_costs()` - Estimate processing costs
- `process_calldata()` - Returns empty (no traditional calldata for Kaspa)

### InterchainSecurityModule (`KaspaMultisigIsm`)
- `module_type()` - Get ISM module type
- `dry_run_verify()` - Dry run ISM verification

### MultisigIsm (`KaspaMultisigIsm`)
- `validators_and_threshold()` - Get validators and threshold for a message

### Indexer (`KaspaMessageIndexer`, `KaspaDeliveryIndexer`)
- `fetch_logs_in_range()` - Fetch messages/deliveries in DAA score range
- `get_finalized_block_number()` - Get finalized DAA score

### SequenceAwareIndexer
- `latest_sequence_count_and_tip()` - Get latest sequence and tip

## RPC Endpoints Used

The client communicates with igra-service via these endpoints:

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rpc/mailbox/count` | Get message count |
| GET | `/rpc/mailbox/delivered/{id}` | Check delivery status |
| GET | `/rpc/mailbox/default_ism` | Get default ISM |
| POST | `/rpc/mailbox/estimate_costs` | Estimate processing costs |
| POST | `/rpc` (JSON-RPC) | `hyperlane.mailbox_process` - Process message |
| GET | `/rpc/ism/module_type` | Get ISM module type |
| POST | `/rpc/ism/dry_run_verify` | Dry run verification |
| POST | `/rpc` (JSON-RPC) | `hyperlane.validators_and_threshold` - Get validators |
| GET | `/rpc/indexer/finalized_block` | Get finalized DAA score |
| GET | `/rpc/indexer/deliveries` | Get deliveries in range |
| GET | `/rpc/indexer/messages` | Get messages in range |
| GET | `/rpc/indexer/sequence_tip` | Get sequence tip |
| GET | `/rpc/chain/info` | Get chain info |
| GET | `/rpc/chain/block/{daa_score}` | Get block by DAA score |
| GET | `/rpc/chain/balance/{address}` | Get balance |
| GET | `/rpc/chain/transaction/{hash}` | Get transaction |

## Features Not Supported

Due to Kaspa's unique architecture, some Hyperlane features work differently:

| Feature | Status | Notes |
|---------|--------|-------|
| MerkleTreeHook | Not applicable | igra-service handles merkle proofs internally |
| ValidatorAnnounce | Not applicable | Validator config is in igra-service |
| InterchainGasPaymaster | Not applicable | Gas is handled by igra-service |
| External signers | Not applicable | Signing is done by igra-service threshold group |
| RoutingIsm | Not implemented yet | Could be added if needed |
| AggregationIsm | Not implemented yet | Could be added if needed |
| CcipReadIsm | Not supported | Not applicable for Kaspa |

## Upstream compatibility

This branch merges `hyperlane-xyz/main` at `4cdfc0ca8` into Reshmem's
Kaspa-enabled `devel` branch. Kaspa remains a destination-only integration
using the classic submitter; Lander rejects Kaspa explicitly. Upstream's
Tron support remains enabled. Chain metrics expose the virtual DAA score
as `ChainInfo.block_height`.

The Rust workspaces pin UUID to 1.16.0 so consumers can combine
`hyperlane-core` with the Kaspa 2.1 SDK. The adapter itself communicates
with Igeret over HTTP and does not link the Kaspa SDK.

## Testing

Run from `rust/main`:

```bash
cargo test --locked -p hyperlane-kaspa
cargo test --locked -p hyperlane-base --features kaspa kaspa_
cargo check --locked -p relayer --features kaspa
```

Build validator and relayer separately: Kaspa is a destination-only feature
and validator's origin configuration does not support it.

```bash
cargo build --release --locked -p validator
cargo build --release --locked -p relayer --features kaspa
```

Unit tests cover configuration validation and the chain-info HTTP response.
End-to-end message delivery requires Igeret, a Kaspa node, an EVM origin,
validators, and a relayer; it is exercised by Igeret's devnet harness.

## Related Documentation

- [HYPERLANE-INTEGRATION.md](../../../igra/HYPERLANE-INTEGRATION.md) - Full integration guide
- [igra-service](../../../igra/) - Threshold signing service for Kaspa
