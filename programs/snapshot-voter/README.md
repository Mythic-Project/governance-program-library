# Snapshot Voter Program

The Snapshot Voter Program is a Solana program that enables Realms (spl-gov) DAOs to implement snapshot-based voting mechanisms using merkle proofs for voter weight verification.

## Overview

The program implements a voting system with three main components:

1. Registrar Creation
2. Registrar Updates
3. Voter Weight Updates

## Key Components

### Registrar Creation

The registrar is the foundational component of the voting system and must be initialized by a governance DAO. This establishment creates the base infrastructure for the snapshot-based voting mechanism.

### Registrar Updates

Registrar updates are proposal-specific and follow these rules:
- Can only be performed when a proposal is in Draft status
- Two proposals must be active simultaneously
- Each update includes:
  - A merkle root
  - An immutable URI pointing to off-chain root data for verification (i.e. Arweave, IPFS, etc)
- Updates must be approved or declined by the DAO
```rust
pub fn update_registrar(
    ctx: Context<UpdateRegistrar>,
    root: [u8; 32],
    uri: Option<String>,
) -> Result<()> 
```

### Voter Weight Updates

Voter weight updates are required for participation in official votes:
- Must be called before each vote
- Occurs after a successful registrar update
- Implementation structure:
```rust
pub fn update_voter_weight_record(
    ctx: Context<UpdateVoterWeightRecord>,
    amount: u64,
    verification_data: Vec<u8>,
) -> Result<()>
```

## Technical Flow

1. DAO creates initial registrar (`create_registrar`)
2. For each proposal:
   - Submit registrar update with new merkle root and URI
   - DAO approves/declines update
   - Voters call `update_voter_weight` with verification data
   - Proceed with voting