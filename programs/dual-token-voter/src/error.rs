use anchor_lang::prelude::*;

#[error_code]
pub enum DualTokenVoterError {
    #[msg("Invalid realm authority")]
    InvalidRealmAuthority,

    #[msg("Invalid amount - must be greater than zero")]
    InvalidAmount,

    #[msg("Math overflow")]
    Overflow,

    #[msg("Division error")]
    DivisionError,

    #[msg("Insufficient deposit")]
    InsufficientDeposit,

    #[msg("Exchange rate is required for CreateProposal action")]
    ExchangeRateRequired,

    #[msg("Invalid snapshot for this proposal")]
    InvalidSnapshot,

    #[msg("Not eligible to vote - tokens updated too close to proposal creation")]
    NotEligibleToVote,

    #[msg("Math overflow during calculation")]
    MathOverflow,

    #[msg("Unsupported voter weight action")]
    UnsupportedAction,

    #[msg("Exchange rate snapshot already exists for this proposal")]
    SnapshotAlreadyExists,

    // Security validations from audit
    #[msg("Eligibility window must be at least 1 hour (3600 seconds)")]
    EligibilityWindowTooSmall,

    #[msg("Eligibility window cannot exceed 30 days (2592000 seconds)")]
    EligibilityWindowTooLarge,

    #[msg("Exchange rate out of bounds")]
    InvalidExchangeRate,

    #[msg("Token A and Token B mints cannot be the same")]
    SameTokenMints,

    #[msg("Missing required token account")]
    MissingTokenAccount,

    #[msg("Wrong token mint")]
    WrongTokenMint,

    // Orca math errors
    #[msg("Liquidity overflow")]
    LiquidityOverflow,

    #[msg("Liquidity underflow")]
    LiquidityUnderflow,

    #[msg("Liquidity too high")]
    LiquidityTooHigh,

    #[msg("Multiplication overflow")]
    MultiplicationOverflow,

    #[msg("Token max exceeded")]
    TokenMaxExceeded,

    #[msg("Token min subceeded")]
    TokenMinSubceeded,

    #[msg("Divide by zero")]
    DivideByZero,

    #[msg("Square root price out of bounds")]
    SqrtPriceOutOfBounds,

    #[msg("Multiplication shift right overflow")]
    MultiplicationShiftRightOverflow,

    #[msg("Number down cast error")]
    NumberDownCastError,

    #[msg("Mul div overflow")]
    MulDivOverflow,

    // Additional Orca swap errors
    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid whirlpool account")]
    InvalidWhirlpool,

    #[msg("Invalid whirlpool mint")]
    InvalidWhirlpoolMint,

    #[msg("No liquidity in pool")]
    NoLiquidity,

    #[msg("Invalid timestamp")]
    InvalidTimestamp,

    #[msg("Trade is not enabled on this pool")]
    TradeNotEnabled,

    #[msg("Exchange rate calculation failed")]
    ExchangeRateCalculationFailed,

    #[msg("Invalid tick array sequence")]
    InvalidTickArraySequence,

    #[msg("Tick array index out of bounds")]
    TickArrayIndexOutofBounds,

    #[msg("Tick array sequence invalid index")]
    TickArraySequenceInvalidIndex,

    #[msg("Invalid sqrt price limit direction")]
    InvalidSqrtPriceLimitDirection,

    #[msg("Zero tradable amount")]
    ZeroTradableAmount,

    #[msg("Amount remaining overflow")]
    AmountRemainingOverflow,

    #[msg("Amount calculation overflow")]
    AmountCalcOverflow,

    #[msg("Partial fill error")]
    PartialFillError,

    #[msg("Invalid adaptive fee constants")]
    InvalidAdaptiveFeeConstants,

    #[msg("Liquidity net error")]
    LiquidityNetError,

    #[msg("Liquidity is zero")]
    LiquidityZero,

    #[msg("Invalid tick spacing")]
    InvalidTickSpacing,

    #[msg("Different whirlpool tick array account")]
    DifferentWhirlpoolTickArrayAccount,

    #[msg("Amount out below minimum")]
    AmountOutBelowMinimum,

    #[msg("Amount in above maximum")]
    AmountInAboveMaximum,

    #[msg("Invalid token mint order")]
    InvalidTokenMintOrder,

    #[msg("Invalid reward index")]
    InvalidRewardIndex,

    #[msg("Fee rate exceeds maximum")]
    FeeRateMaxExceeded,

    #[msg("Protocol fee rate exceeds maximum")]
    ProtocolFeeRateMaxExceeded,

    #[msg("Invalid start tick")]
    InvalidStartTick,

    #[msg("Tick not found")]
    TickNotFound,
}
