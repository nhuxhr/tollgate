use anchor_lang::prelude::*;

#[error_code]
pub enum TollgateError {
    // Invalid inputs
    #[msg("The provided pool is not a valid DAMM v2 pool")]
    InvalidPool,
    #[msg("The provided pool config is not a valid DAMM v2 pool config")]
    InvalidPoolConfig,
    #[msg("The provided position is not a valid DAMM v2 position")]
    InvalidPosition,
    #[msg("Base mint not found in the provided pool")]
    BaseMintNotInPool,
    #[msg("Quote mint not found in the provided pool")]
    QuoteMintNotInPool,
    #[msg("Base and quote mints are the same")]
    BaseAndQuoteMintsAreSame,
    #[msg("Invalid investor accounts")]
    InvalidInvestorAccounts,
    #[msg("Invalid investor ATA")]
    InvalidInvestorAta,

    // Mismatched configurations
    #[msg("The provided pool does not match the provided pool config")]
    PoolConfigMismatch,
    #[msg("The provided pool is not in quote-only fee mode")]
    PoolNotQuoteOnlyFees,
    #[msg("The provided pool config is not in quote-only fee mode")]
    PoolConfigNotQuoteOnlyFees,
    #[msg("The provided AMM program does not match the expected DAMM v2 program")]
    AMMProgramMismatch,

    // Invalid states
    #[msg("Invalid day state")]
    InvalidDayState,
    #[msg("Base denominated fees are not allowed")]
    BaseDenominatedFees,

    // Invalid operations
    #[msg("Cannot start a new day yet")]
    CannotStartNewDay,
    #[msg("Cannot continue the same day")]
    CannotContinueSameDay,
    #[msg("Cannot close the day yet")]
    CannotCloseDay,

    // Invalid parameters
    #[msg("The provided investor count is invalid or zero")]
    InvalidInvestors,
    #[msg("The provided investor fee share BPS is invalid or out of range")]
    InvalidInvestorFeeShareBps,
    #[msg("The provided minimum payout lamports is invalid")]
    InvalidMinPayoutLamports,
    #[msg("The provided daily cap is invalid")]
    InvalidDailyCap,
    #[msg("The provided Y0 allocation is invalid")]
    InvalidY0Allocation,
    #[msg("Pagination cursor is too small")]
    PaginationCursorTooSmall,
    #[msg("Pagination cursor is too large")]
    PaginationCursorTooLarge,
    #[msg("Cursor exceeds the number of investors")]
    CursorExceedsInvestors,

    // Initialization errors
    #[msg("The provided policy has already been initialized")]
    PolicyAlreadyInitialized,
    #[msg("The provided progress has already been initialized")]
    ProgressAlreadyInitialized,
}
