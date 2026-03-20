# Core Attribute Voter

An SPL Governance voter weight addin that enables attribute-based governance voting using [Metaplex Core](https://developers.metaplex.com/core) NFT assets.

## Overview

Core Attribute Voter allows DAOs to use NFT collections for governance voting where each NFT's voting power is determined dynamically from its on-chain attributes. Different NFTs within the same collection can carry different voting power based on traits, tiers, or any other attribute stored in the Metaplex Core Attributes plugin.

## How It Works

### Weight Calculation

Each configured collection specifies:
- A **weight attribute key** (e.g. `"voting_power"`, `"tier"`) — the attribute name to read from each NFT
- A **max weight** — a ceiling that caps any single NFT's voting power
- A **total weight** — the collection's contribution to the quorum denominator (`max_voter_weight`)
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
max_voter_weight = Σ collection.total_weight
```

This is used by SPL Governance to calculate quorum thresholds. Unlike the nft-voter and core-voter plugins (where max weight = `collection_size × weight_per_nft`), attribute-based voting has variable per-NFT weights, so `total_weight` must be set by the realm authority to reflect the expected total voting power of each collection.

#### Setting `max_weight` and `total_weight` correctly

- `max_weight` caps per-NFT voting power.
- `total_weight` controls quorum denominator contribution for the collection.

**Example 1 — Well-calibrated:**
A collection of 50 NFTs where attributes range from 1–10, totalling ~200 across the collection.
Setting `max_weight = 200` means:
- Individual NFTs are capped at 200 (effectively uncapped since max attribute is 10)
- If `total_weight = 200`, quorum denominator reflects the true total voting power
- A 60% quorum requires 120 voting power to pass

**Example 2 — Set too low:**
Same collection, but `max_weight = 50`.
- Individual NFTs with attribute > 50 get capped (unlikely here, but enforced)
- Quorum denominator is only 50, so just 30 voting power (60% quorum) passes a proposal
- Risk: a small minority of NFT holders can pass proposals

**Example 3 — Set too high:**
Same collection, but `max_weight = 10000`.
- No individual capping (attributes are far below 10000)
- Quorum denominator is 10000, so 6000 voting power needed for 60% quorum
- Risk: quorum becomes unreachable since the collection only holds ~200 total power

**Example 4 — Multiple collections:**
Collection A: `max_weight = 500`, `total_weight = 500`; Collection B: `max_weight = 300`, `total_weight = 300`.
- `max_voter_weight = 500 + 300 = 800` (sum of `total_weight`)
- A voter holding NFTs from both collections accumulates weight across them
- 60% quorum requires 480 total voting power

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
    total_weight: 1000,
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
| `max_weight` | `u64` | > 0 | Max governance weight per NFT (attribute cap per asset). |
| `total_weight` | `u64` | > 0 | Collection's total governance contribution for quorum calculation. Summed across collections into `max_voter_weight`. |
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
