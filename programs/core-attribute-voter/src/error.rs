use anchor_lang::prelude::*;

#[error_code]
pub enum CoreNftAttributeVoterError {
    // 0
    #[msg("Invalid Realm Authority")]
    InvalidRealmAuthority,

    #[msg("Invalid Realm for Registrar")]
    InvalidRealmForRegistrar,

    #[msg("Invalid max weight, must be greater than 0")]
    InvalidMaxWeight,

    #[msg("Invalid MaxVoterWeightRecord Realm")]
    InvalidMaxVoterWeightRecordRealm,

    #[msg("Invalid MaxVoterWeightRecord Mint")]
    InvalidMaxVoterWeightRecordMint,

    #[msg("CastVote Is Not Allowed")]
    CastVoteIsNotAllowed,

    #[msg("Invalid VoterWeightRecord Realm")]
    InvalidVoterWeightRecordRealm,

    #[msg("Invalid VoterWeightRecord Mint")]
    InvalidVoterWeightRecordMint,

    #[msg("Invalid TokenOwner for VoterWeightRecord")]
    InvalidTokenOwnerForVoterWeightRecord,

    #[msg("Collection must be verified")]
    CollectionMustBeVerified,

    //10
    #[msg("Voter does not own NFT")]
    VoterDoesNotOwnNft,

    #[msg("Collection not found")]
    CollectionNotFound,

    #[msg("Missing Metadata collection")]
    MissingMetadataCollection,

    #[msg("Token Metadata doesn't match")]
    TokenMetadataDoesNotMatch,

    #[msg("Invalid account owner")]
    InvalidAccountOwner,

    #[msg("Invalid token metadata account")]
    InvalidTokenMetadataAccount,

    #[msg("Duplicated NFT detected")]
    DuplicatedNftDetected,

    #[msg("Invalid NFT amount")]
    InvalidNftAmount,

    #[msg("NFT already voted")]
    NftAlreadyVoted,

    #[msg("Invalid Proposal for NftVoteRecord")]
    InvalidProposalForNftVoteRecord,

    // 20
    #[msg("Invalid TokenOwner for NftVoteRecord")]
    InvalidTokenOwnerForNftVoteRecord,

    #[msg("VoteRecord must be withdrawn")]
    VoteRecordMustBeWithdrawn,

    #[msg("Invalid VoteRecord for NftVoteRecord")]
    InvalidVoteRecordForNftVoteRecord,

    #[msg("VoterWeightRecord must be expired")]
    VoterWeightRecordMustBeExpired,

    #[msg("Invalid NFT collection")]
    InvalidNftCollection,

    #[msg("Proposal is not in voting state")]
    InvalidProposalState,

    #[msg("Attribute not found on asset")]
    AttributeNotFound,

    #[msg("Invalid attribute value, must be a valid u64")]
    InvalidAttributeValue,

    #[msg("Invalid weight attribute key")]
    InvalidWeightAttributeKey,

    #[msg("Attribute plugin authority mismatch")]
    AttributeAuthorityMismatch,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("Failed to borrow VoteRecord data")]
    VoteRecordBorrowFailed,

    #[msg("Invalid GoverningTokenMint for Proposal")]
    InvalidGoverningTokenMintForProposal,
}
