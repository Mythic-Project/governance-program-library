# Core Attribute Voter

An SPL Governance voter weight addin that enables attribute-based governance voting using [Metaplex Core](https://developers.metaplex.com/core) NFT assets.

## Overview

Core Attribute Voter allows DAOs to use NFT collections for governance voting where each NFT's voting power is determined dynamically from its on-chain attributes. Different NFTs within the same collection can carry different voting power based on traits, tiers, or any other attribute stored in the Metaplex Core Attributes plugin.

## How It Works

### Weight Calculation

Each configured collection specifies:
- A **weight attribute key** (e.g. `"voting_power"`, `"tier"`) — the attribute name to read from each NFT
- A **max weight** — a ceiling that caps any single NFT's voting power
- An **expected attribute authority** — the trusted authority that set the attributes

When a voter submits their NFTs, the program:
1. Reads the Attributes plugin from the Metaplex Core asset
2. Verifies the plugin authority matches the expected authority
3. Finds the attribute matching the configured key
4. Parses the value as a `u64`
5. Caps it to `min(attribute_value, max_weight)`

**Example:** A collection configured with `max_weight = 100` and `weight_attribute_key = "voting_power"`:
- NFT with `voting_power: "75"` → weight = 75
- NFT with `voting_power: "150"` → weight = 100 (capped)
- Voter holding both → total weight = 175

### Max Voter Weight

The maximum possible voting power across all configured collections:

```
max_voter_weight = Σ (collection.size × collection.max_weight)
```

This is used by SPL Governance to calculate quorum thresholds.

## Architecture

### Accounts

| Account | Seeds | Purpose |
|---|---|---|
| **Registrar** | `["registrar", realm, governing_token_mint]` | Stores collection configs for a realm |
| **VoterWeightRecord** | `["voter-weight-record", realm, governing_token_mint, governing_token_owner]` | Per-voter weight used by SPL Governance |
| **MaxVoterWeightRecord** | `["max-voter-weight-record", realm, governing_token_mint]` | Maximum possible weight for quorum |
| **AssetVoteRecord** | `["nft-vote-record", proposal, asset_mint]` | Prevents same NFT from voting twice on a proposal |

### Instructions

| Instruction | Signer | Description |
|---|---|---|
| `create_registrar` | Realm authority | Creates the registrar for a realm, pre-allocates collection slots |
| `configure_collection` | Realm authority | Adds or updates an NFT collection config on the registrar |
| `create_max_voter_weight_record` | Payer | Creates the max voter weight record for a realm |
| `update_max_voter_weight_record` | Anyone | Refreshes max voter weight from current collection configs |
| `create_voter_weight_record` | Payer | Creates a voter's weight record |
| `update_voter_weight_record` | Voter | Updates weight for non-voting actions (CreateProposal, CreateGovernance, etc.) |
| `cast_nft_vote` | Voter | Casts a vote on a proposal using NFTs, creates AssetVoteRecords |
| `relinquish_nft_vote` | Voter | Cleans up AssetVoteRecords after voting ends, recovers rent |

## Usage Flow

### 1. Setup (Realm Authority)

```
create_registrar(max_collections: 3)
    → Registrar PDA created

create_max_voter_weight_record()
    → MaxVoterWeightRecord PDA created

configure_collection(
    max_weight: 100,
    weight_attribute_key: "voting_power",
    expected_attribute_authority: UpdateAuthority
)
    → Collection added to Registrar
    → MaxVoterWeightRecord updated
```

### 2. Voter Registration

```
create_voter_weight_record(governing_token_owner)
    → VoterWeightRecord PDA created (expired, weight = 0)
```

### 3. Non-Voting Actions (CreateProposal, CreateGovernance, etc.)

```
update_voter_weight_record(action: CreateProposal)
    → Reads NFTs from remaining_accounts (up to 5 per tx)
    → Calculates weight from attributes
    → Sets expiry to current slot

[same tx] spl_governance::create_proposal(...)
    → Governance reads VoterWeightRecord
```

### 4. Casting Votes

`cast_nft_vote` can be called multiple times per proposal to accumulate weight across transactions (useful when a voter holds more NFTs than fit in a single transaction).

```
cast_nft_vote()
    → remaining_accounts: [nft1, vote_record1, nft2, vote_record2, ...]
    → For each NFT: validate ownership, read weight, create AssetVoteRecord
    → Accumulates weight in VoterWeightRecord
    → Sets action = CastVote, target = proposal

[final tx] spl_governance::cast_vote(...)
    → Governance reads VoterWeightRecord
    → Vote recorded
```

### 5. Relinquishing Votes

After the proposal's voting period ends or the voter withdraws their vote:

```
relinquish_nft_vote()
    → remaining_accounts: [asset_vote_record1, asset_vote_record2, ...]
    → Disposes AssetVoteRecords, recovers rent
    → Resets VoterWeightRecord weight to 0
```

## Configuration

### Collection Config Parameters

| Parameter | Type | Constraints | Description |
|---|---|---|---|
| `max_weight` | `u64` | Any value | Per-NFT weight ceiling |
| `weight_attribute_key` | `String` | 1–32 characters | Attribute name to read from NFTs |
| `expected_attribute_authority` | `PluginAuthority` | Must match plugin | Trusted authority for attribute validation |

### Limits

| Limit | Value | Reason |
|---|---|---|
| Collections per registrar | Up to 255 (`u8`) | Set at registrar creation |
| Weight attribute key length | 1–32 chars | Enforced in `configure_collection` |
| NFTs per `update_voter_weight_record` | ~5 | Solana transaction size limit |
| NFTs per `cast_nft_vote` | Unlimited | Call multiple times to accumulate |

## Security Considerations

- **Attribute authority validation** — The program verifies that the Attributes plugin authority on each NFT matches the `expected_attribute_authority` configured for the collection. This prevents anyone from setting arbitrary weight attributes.
- **Double-vote prevention** — `AssetVoteRecord` PDAs (seeded by proposal + asset) ensure the same NFT cannot vote twice on the same proposal.
- **Expiry enforcement** — `VoterWeightRecord` expires at the slot it was set, so it can only be consumed in the same transaction. This prevents stale weights from being reused.
- **Checked arithmetic** — All weight calculations use checked math to prevent overflow panics.
- **Relinquish guards** — Votes can only be relinquished after the voting period ends or the spl-gov VoteRecord is withdrawn, and only when the VoterWeightRecord is expired (prevents front-running attacks with stacked voter-weight plugins).

## Building

```sh
anchor build -p gpl_core_attribute_voter
```

## Testing

```sh
cargo test-sbf -p gpl-core-attribute-voter
```
