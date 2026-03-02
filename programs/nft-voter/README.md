# NFT Voter

SPL Governance addin implementing Metaplex Token Metadata NFT-based governance. Allows NFT holders to participate in DAO governance using their NFTs as voting power.

Program ID: `GnftV5kLjd67tvHpNGyodwWveEKivz3ZWvvE3Z4xi2iw`

## Overview

The NFT Voter plugin integrates with spl-governance as a voter weight addin. It enables DAOs to use NFT collections as the basis for voting power instead of (or in addition to) fungible tokens.

### Instructions

| Instruction | Description |
|---|---|
| `create_registrar` | Creates the NFT voting registrar for a realm |
| `configure_collection` | Configures an NFT collection with a vote weight and size |
| `create_voter_weight_record` | Creates a voter weight record for a governing token owner |
| `create_max_voter_weight_record` | Creates the max voter weight record for the registrar |
| `update_voter_weight_record` | Updates voter weight for non-CastVote actions |
| `cast_nft_vote` | Casts an NFT vote (voter pays rent for vote records) |
| `relinquish_nft_vote` | Disposes NFT vote records and returns rent to a beneficiary |
| `create_sponsor` | Creates and funds a Sponsor PDA for sponsored voting |
| `withdraw_sponsor` | Withdraws SOL from the Sponsor PDA (realm authority only) |
| `cast_nft_vote_sponsored` | Casts an NFT vote with rent paid by the Sponsor PDA |
| `relinquish_nft_vote_sponsored` | Disposes sponsored vote records and returns rent to the Sponsor PDA |

### Accounts

| Account | Description |
|---|---|
| `Registrar` | Stores NFT voting configuration (realm, governance program, collection configs) |
| `VoterWeightRecord` | Stores computed voter weight for spl-governance consumption |
| `MaxVoterWeightRecord` | Stores the maximum possible voter weight for the registrar |
| `NftVoteRecord` | Tracks that an NFT was used to vote on a proposal (voter-paid rent) |
| `NftVoteRecordSponsored` | Tracks that an NFT was used to vote on a proposal (sponsor-paid rent) |
| `Sponsor` | SystemAccount PDA that holds SOL to fund sponsored vote records |

## Changelog

### 0.3.0 - Sponsored Voting

Adds sponsored voting support, allowing DAOs to subsidize the rent costs of NFT vote records so voters don't need SOL to participate.

#### New Instructions

- **`create_sponsor`** - Creates a Sponsor PDA (`seeds = ["sponsor", registrar]`) and funds it with the rent-exempt minimum from the payer. Requires the realm authority to sign, validated against the on-chain Realm account.

- **`withdraw_sponsor`** - Withdraws SOL from the Sponsor PDA to any destination account. Requires realm authority signature. Enforces a floor at `rent.minimum_balance(0)` to keep the account alive.

- **`cast_nft_vote_sponsored`** - Same validation as `cast_nft_vote` (NFT ownership, collection membership, duplicate detection) but rent for `NftVoteRecordSponsored` accounts is paid from the Sponsor PDA instead of the voter. Pre-checks that the sponsor has sufficient funds before creating any records. Supports accumulative voting across multiple transactions.

- **`relinquish_nft_vote_sponsored`** - Disposes `NftVoteRecordSponsored` accounts and returns rent to the Sponsor PDA (not an arbitrary beneficiary). Validates that each record's stored `sponsor` field matches the passed Sponsor account, ensuring rent flows back to the correct source. Includes the same anti-sandwich and vote-record-withdrawal checks as `relinquish_nft_vote`.

#### New Accounts

- **`NftVoteRecordSponsored`** - Similar to `NftVoteRecord` but includes a `sponsor: Pubkey` field that records which Sponsor PDA paid the rent. Uses a distinct discriminator (`sha256("account:NftVoteRecordSponsored")[..8]`) to distinguish it from `NftVoteRecord`.

- **Sponsor PDA** - A SystemAccount PDA derived from `["sponsor", registrar]`. Being system-owned (no program data) allows it to use `system_instruction::create_account` and `system_instruction::transfer` via `invoke_signed`. Anyone can send SOL to it; only the program can spend from it via CPI.

#### Design Decisions

- **SystemAccount PDA pattern**: The Sponsor is a system-owned PDA rather than a program-owned account with data. This avoids the Solana runtime restriction that `system_instruction::transfer` requires `from` to be system-owned. The nft-voter program can still sign for the PDA via `invoke_signed` since it derives the address.

- **Shared PDA namespace**: `NftVoteRecordSponsored` uses the same PDA seeds as `NftVoteRecord` (`["nft-vote-record", proposal, nft_mint]`). This prevents cross-instruction double voting -- if a voter casts via `cast_nft_vote`, they cannot also cast via `cast_nft_vote_sponsored` with the same NFT on the same proposal (and vice versa), because the PDA is already occupied. Discriminators ensure each record type can only be relinquished through its corresponding instruction.

- **Realm authority gating**: Both `create_sponsor` and `withdraw_sponsor` require the realm authority to sign, validated by deserializing the on-chain Realm account and comparing `realm.authority`. This ensures only the DAO's authority can manage sponsor funds.
