# Tollgate Program

<!--toc:start-->

- [Tollgate Program](#tollgate-program)
  - [Introduction](#introduction)
  - [Integration Steps](#integration-steps)
    - [Overview](#overview)
    - [Step 1: Initialize](#step-1-initialize)
    - [Step 2: Crank](#step-2-crank)
  - [Account Structures](#account-structures)
    - [Policy Account](#policy-account)
    - [Progress Account](#progress-account)
  - [Error Codes](#error-codes)
  - [Day & Pagination Semantics](#day-pagination-semantics)
    - [Day State](#day-state)
    - [Pagination Cursor](#pagination-cursor)
    - [Page Size](#page-size)
    - [Page Payouts](#page-payouts)
    - [Pagination Flow Diagram](#pagination-flow-diagram)
  - [Events](#events) - [HonoraryPositionInitialized](#honorarypositioninitialized) - [QuoteFeesClaimed](#quotefeesclaimed) - [InvestorPayoutPage](#investorpayoutpage) - [CreatorPayoutDayClosed](#creatorpayoutdayclosed)

<!--toc:end-->

## Introduction

The Tollgate program is designed to work with the DAMM v2 protocol, allowing for the creation of honorary positions and the distribution of fees to investors. This program is built for the Superteam bounty: Build Permissionless Fee Routing Anchor Program for Meteora DLMM v2 (see <https://earn.superteam.fun/listing/build-permissionless-fee-routing-anchor-program-for-meteora-dlmm-v2> for details).

## Integration Steps

### Overview

The integration steps for the Tollgate program involve initializing the policy and progress accounts, and cranking the daily distribution.

### Step 1: Initialize

The `initialize` instruction is used to initialize the policy and progress accounts, and create a DAMM v2 position.

| **Parameter**            | **Type**      | **Description**                                              |
| ------------------------ | ------------- | ------------------------------------------------------------ |
| `init_investor_ata`      | `bool`        | A boolean indicating whether to initialize the investor ATA. |
| `investor_fee_share_bps` | `u16`         | The investor fee share BPS.                                  |
| `min_payout_lamports`    | `u64`         | The minimum payout lamports.                                 |
| `daily_cap`              | `Option<u64>` | The daily cap.                                               |
| `y0`                     | `u64`         | The Y0 allocation.                                           |

| Account                | Constraint                          | Description                                                                      |
| ---------------------- | ----------------------------------- | -------------------------------------------------------------------------------- |
| `vault`                | `signer`                            | The signer account that will be used to create the policy and progress accounts. |
| `policy`               | `init`, `PDA`                       | The policy account that will be initialized.                                     |
| `progress`             | `init`, `PDA`                       | The progress account that will be initialized.                                   |
| `pool`                 | `mut`, `constraint = is_valid_pool` | The DAMM v2 pool account that must be valid.                                     |
| `pool_cfg`             | `constraint = is_valid_pool_cfg`    | The pool configuration account that must be valid.                               |
| `position_nft_mint`    | `mut`, `signer`                     | The mint account for the position NFT.                                           |
| `position_nft_account` | `mut`, `PDA`                        | The account that will hold the position NFT.                                     |
| `position`             | `mut`, `PDA`                        | The DAMM v2 pool position account.                                               |
| `pool_authority`       | -                                   | The pool authority account.                                                      |
| `owner`                | `PDA`                               | The system account that owns the vault.                                          |
| `quote_mint`           | -                                   | The quote mint account.                                                          |
| `payer`                | `mut`, `signer`                     | The signer account that will pay for the initialization.                         |
| `event_authority`      | -                                   | The DAMM v2 event authority account.                                             |
| `amm_program`          | `address = damm_v2::ID`             | The DAMM v2 AMM program account.                                                 |
| `token_2022_program`   | -                                   | The Token 2022 program account.                                                  |
| `system_program`       | -                                   | The system program account.                                                      |

```rust
// Initialize the policy and progress accounts, and create a DAMM v2 position
let initialize_instruction = Instruction::new_with_borsh(
    tollgate::ID,
    &tollgate::accounts::AccountInitialize {
        vault: vault_account,
        policy: policy_account,
        progress: progress_account,
        pool: pool_account,
        pool_cfg: pool_cfg_account,
        position_nft_mint: position_nft_mint_account,
        position_nft_account: position_nft_account,
        position: position_account,
        pool_authority: pool_authority_account,
        owner: owner_account,
        quote_mint: quote_mint_account,
        payer: payer_account,
        event_authority: event_authority_account,
        amm_program: amm_program_account,
        token_2022_program: token_2022_program_account,
        system_program: system_program_account,
   },
    tollgate::instruction::InitializeParams {
        init_investor_ata: true,
        investor_fee_share_bps: 5000,
        min_payout_lamports: 1000000,
        daily_cap: Some(10000000),
        y0: 100000,
    },
);
```

### Step 2: Crank

The `crank` instruction is used to crank the daily distribution.

| **Parameter** | **Type** | **Description**                                            |
| ------------- | -------- | ---------------------------------------------------------- |
| `cursor`      | `u32`    | The cursor that will be used to paginate the investors.    |
| `page_size`   | `u32`    | The page size that will be used to paginate the investors. |

| Account                    | Constraint                                            | Description                                           |
| -------------------------- | ----------------------------------------------------- | ----------------------------------------------------- |
| `policy`                   | `PDA`                                                 | The policy account.                                   |
| `progress`                 | `PDA`                                                 | The progress account.                                 |
| `pool`                     | `constraint = is_valid_pool`                          | The DAMM v2 pool account that must be valid.          |
| `position_nft_account`     | `token::authority = owner`                            | The position NFT account.                             |
| `position`                 | `mut`, `has_one = pool`                               | The DAMM v2 pool position account.                    |
| `pool_authority`           | -                                                     | The pool authority account.                           |
| `owner`                    | `PDA`                                                 | The system account that owns the vault.               |
| `treasury`                 | `init_if_needed`                                      | The treasury account.                                 |
| `base_account`             | `init_if_needed`                                      | The owner base account.                               |
| `quote_account`            | `init_if_needed`                                      | The owner quote account.                              |
| `base_vault`               | `mut`, `token::token_program = base_program`          | The base vault account.                               |
| `quote_vault`              | `mut`, `token::token_program = quote_program`         | The quote vault account.                              |
| `base_mint`                | -                                                     | The base mint account.                                |
| `quote_mint`               | -                                                     | The quote mint account.                               |
| `base_program`             | -                                                     | The base token program account.                       |
| `quote_program`            | -                                                     | The quote token program account.                      |
| `creator_account`          | `mut`, `associated_token::authority = policy.creator` | The creator account.                                  |
| `payer`                    | `mut`                                                 | The signer account that will pay for the instruction. |
| `event_authority`          | -                                                     | The DAMM v2 event authority account.                  |
| `amm_program`              | `address = damm_v2::ID`                               | The DAMM v2 AMM program account.                      |
| `associated_token_program` | -                                                     | The associated token program account.                 |
| `system_program`           | -                                                     | The system program account.                           |

```rust
// Crank the daily distribution
let crank_instruction = Instruction::new_with_borsh(
    tollgate::ID,
    &tollgate::accounts::AccountCrank {
        policy: policy_account,
        progress: progress_account,
        pool: pool_account,
        position_nft_account: position_nft_account,
        position: position_account,
        pool_authority: pool_authority_account,
        owner: owner_account,
        treasury: treasury_account,
        base_account: base_account,
        quote_account: quote_account,
        base_vault: base_vault_account,
        quote_vault: quote_vault_account,
        base_mint: base_mint_account,
        quote_mint: quote_mint_account,
        base_program: base_program_account,
        quote_program: quote_program_account,
        creator_account: creator_account,
        payer: payer_account,
        event_authority: event_authority_account,
        amm_program: amm_program_account,
        associated_token_program: associated_token_program_account,
        system_program: system_program_account,
    },
    tollgate::instruction::CrankParams {
        cursor: 0,
        page_size: 10,
    },
);
```

## Account Structures

The Tollgate program uses the following account structures:

### Policy Account

The policy account is used to store the policy state.

| Field                    | Type          | Description                                                                     |
| ------------------------ | ------------- | ------------------------------------------------------------------------------- |
| `vault`                  | `Pubkey`      | The vault account that will be used to create the policy and progress accounts. |
| `creator`                | `Pubkey`      | The creator account that will receive the remainder of the fees.                |
| `quote_mint`             | `Pubkey`      | The quote mint account that will be used to distribute fees to investors.       |
| `init_investor_ata`      | `bool`        | A boolean indicating whether to initialize the investor ATA.                    |
| `investor_fee_share_bps` | `u16`         | The investor fee share BPS.                                                     |
| `min_payout_lamports`    | `u64`         | The minimum payout lamports.                                                    |
| `daily_cap`              | `Option<u64>` | The daily cap.                                                                  |
| `y0`                     | `u64`         | The Y0 allocation.                                                              |
| `is_initialized`         | `bool`        | Whether the policy is initialized.                                              |
| `owner_bump`             | `u8`          | The owner bump.                                                                 |
| `bump`                   | `u8`          | The bump.                                                                       |

### Progress Account

The progress account is used to store the progress state.

| Field                  | Type       | Description                                                         |
| ---------------------- | ---------- | ------------------------------------------------------------------- |
| `vault`                | `Pubkey`   | The vault account that will be used to create the progress account. |
| `last_distribution_ts` | `i64`      | The timestamp of the last distribution.                             |
| `daily_spent`          | `u64`      | The amount spent in the current day.                                |
| `carry`                | `u64`      | The carryover from the previous day.                                |
| `cursor`               | `u32`      | The cursor that will be used to paginate the investors.             |
| `day_state`            | `DayState` | The day state.                                                      |
| `bump`                 | `u8`       | The bump.                                                           |

## Error Codes

The Tollgate program uses the following error codes:

| Code                       | Group                     | Description                                                           |
| -------------------------- | ------------------------- | --------------------------------------------------------------------- |
| InvalidPool                | Invalid inputs            | The provided pool is not a valid DAMM v2 pool.                        |
| InvalidPoolConfig          | Invalid inputs            | The provided pool config is not a valid DAMM v2 pool config.          |
| InvalidPosition            | Invalid inputs            | The provided position is not a valid DAMM v2 position.                |
| BaseMintNotInPool          | Invalid inputs            | Base mint not found in the provided pool.                             |
| QuoteMintNotInPool         | Invalid inputs            | Quote mint not found in the provided pool.                            |
| BaseAndQuoteMintsAreSame   | Invalid inputs            | Base and quote mints are the same.                                    |
| InvalidInvestorAccounts    | Invalid inputs            | The investor accounts are invalid.                                    |
| InvalidInvestorAta         | Invalid inputs            | The investor ATA is invalid.                                          |
| PoolConfigMismatch         | Mismatched configurations | The provided pool does not match the provided pool config.            |
| PoolNotQuoteOnlyFees       | Mismatched configurations | The provided pool is not in quote-only fee mode.                      |
| PoolConfigNotQuoteOnlyFees | Mismatched configurations | The provided pool config is not in quote-only fee mode.               |
| AMMProgramMismatch         | Mismatched configurations | The provided AMM program does not match the expected DAMM v2 program. |
| InvalidDayState            | Invalid states            | The day state is invalid.                                             |
| BaseDenominatedFees        | Invalid states            | Base denominated fees are not allowed.                                |
| CannotStartNewDay          | Invalid operations        | Cannot start a new day yet.                                           |
| CannotContinueSameDay      | Invalid operations        | Cannot continue the same day.                                         |
| CannotCloseDay             | Invalid operations        | Cannot close the day yet.                                             |
| InvalidInvestors           | Invalid parameters        | The provided investor count is invalid or zero.                       |
| InvalidInvestorFeeShareBps | Invalid parameters        | The provided investor fee share BPS is invalid or out of range.       |
| InvalidMinPayoutLamports   | Invalid parameters        | The minimum payout lamports is invalid.                               |
| InvalidDailyCap            | Invalid parameters        | The daily cap is invalid.                                             |
| InvalidY0Allocation        | Invalid parameters        | The Y0 allocation is invalid.                                         |
| PaginationCursorTooSmall   | Invalid parameters        | The pagination cursor is too small.                                   |
| PaginationCursorTooLarge   | Invalid parameters        | The pagination cursor is too large.                                   |
| PolicyAlreadyInitialized   | Initialization errors     | The policy account has already been initialized.                      |
| ProgressAlreadyInitialized | Initialization errors     | The progress account has already been initialized.                    |

## Day & Pagination Semantics

The Tollgate program uses the following day/pagination semantics:

### Day State

The day state is used to determine whether a new day has started or not.

- **New**: A new day has started.
- **Same**: The same day is continuing.
- **Closed**: The day has been closed.

### Pagination Cursor

The pagination cursor is used to paginate the investors.

- **Cursor**: The cursor that will be used to paginate the investors.

### Page Size

The page size is used to determine the number of investors to distribute fees to per page.

- **Page Size**: The page size that will be used to paginate the investors.

### Page Payouts

The page payouts are the amounts distributed to investors per page.

- **Page Payouts**: The amounts distributed to investors per page.

### Crank Flow Diagram

The following diagram illustrates the crank flow:

```
Start Crank
  |
  v
Check 24h Gate (last_distribution_ts + 86400 <= current_ts)
  |
  v
If New Day:
  - Claim DAMM v2 position fee
  - Update distributable amount
  |
  v
If Distributable < min_payout_lamports:
  - Carry over to next day
  |
  v
Process Investor Payout Page
  - Update daily_spent and cursor
  - Emit InvestorPayoutPage event
  |
  v
If All Investors Processed:
  - Close day
  - Emit CreatorPayoutDayClosed event
  |
  v
End Crank
```

### Pagination Flow Diagram

The following diagram illustrates the pagination flow during the crank instruction:

```
Start Pagination
  |
  v
Get Investor Accounts (remaining_accounts)
  |
  v
Calculate Page Size (min(page_size, remaining investors))
  |
  v
Process Page
  - For each investor in page:
  - Calculate pro-rata share
  - If share >= min_payout: Transfer to investor_ata
  - Update daily_spent and cursor
  |
  v
Check if All Investors Processed
  |
  v
If Yes:
  - Close day
  - Emit CreatorPayoutDayClosed event
  |
  v
If No:
  - Continue to next page
  |
  v
End Pagination
```

## Events

The Tollgate program emits the following events:

### HonoraryPositionInitialized

The honorary position has been initialized.

| Field                    | Type          | Description                                                           |
| ------------------------ | ------------- | --------------------------------------------------------------------- |
| `vault`                  | `Pubkey`      | The vault account that was used to create the position.               |
| `policy`                 | `Pubkey`      | The policy account that was initialized.                              |
| `progress`               | `Pubkey`      | The progress account that was initialized.                            |
| `pool`                   | `Pubkey`      | The pool account that was used to validate the pool.                  |
| `pool_cfg`               | `Pubkey`      | The pool config account that was used to validate the pool config.    |
| `position`               | `Pubkey`      | The position account that was created.                                |
| `owner`                  | `Pubkey`      | The owner account that was used to sign the transaction.              |
| `base_mint`              | `Pubkey`      | The base mint account that was used to create the position NFT.       |
| `quote_mint`             | `Pubkey`      | The quote mint account that was used to distribute fees to investors. |
| `investor_fee_share_bps` | `u16`         | The investor fee share BPS.                                           |
| `min_payout_lamports`    | `u64`         | The minimum payout lamports.                                          |
| `daily_cap`              | `Option<u64>` | The daily cap.                                                        |
| `y0`                     | `u64`         | The Y0 allocation.                                                    |

```rust
#[event]
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub pool_cfg: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub investor_fee_share_bps: u16,
    pub min_payout_lamports: u64,
    pub daily_cap: Option<u64>,
    pub y0: u64,
}
```

### QuoteFeesClaimed

The quote fees have been claimed.

| Field               | Type     | Description                                              |
| ------------------- | -------- | -------------------------------------------------------- |
| `vault`             | `Pubkey` | The vault account that was used to create the position.  |
| `policy`            | `Pubkey` | The policy account that was initialized.                 |
| `progress`          | `Pubkey` | The progress account that was initialized.               |
| `pool`              | `Pubkey` | The pool account that was used to validate the pool.     |
| `position`          | `Pubkey` | The position account that was created.                   |
| `owner`             | `Pubkey` | The owner account that was used to sign the transaction. |
| `base_fee_claimed`  | `u64`    | The base fee that was claimed.                           |
| `quote_fee_claimed` | `u64`    | The quote fee that was claimed.                          |

```rust
#[event]
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub base_fee_claimed: u64,
    pub quote_fee_claimed: u64,
}
```

### InvestorPayoutPage

The investor payout page has been processed.

| Field        | Type     | Description                                              |
| ------------ | -------- | -------------------------------------------------------- |
| `vault`      | `Pubkey` | The vault account that was used to create the position.  |
| `policy`     | `Pubkey` | The policy account that was initialized.                 |
| `progress`   | `Pubkey` | The progress account that was initialized.               |
| `pool`       | `Pubkey` | The pool account that was used to validate the pool.     |
| `position`   | `Pubkey` | The position account that was created.                   |
| `owner`      | `Pubkey` | The owner account that was used to sign the transaction. |
| `cursor`     | `u32`    | The cursor that was used to paginate the investors.      |
| `investors`  | `u32`    | The number of investors that were processed.             |
| `page_start` | `u32`    | The starting page number.                                |
| `page_end`   | `u32`    | The ending page number.                                  |
| `payout`     | `u64`    | The total payout that was processed.                     |

```rust
#[event]
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub cursor: u32,
    pub investors: u32,
    pub page_start: u32,
    pub page_end: u32,
    pub payout: u64,
}
```

### CreatorPayoutDayClosed

The creator payout day has been closed.

| Field               | Type     | Description                                              |
| ------------------- | -------- | -------------------------------------------------------- |
| `vault`             | `Pubkey` | The vault account that was used to create the position.  |
| `policy`            | `Pubkey` | The policy account that was initialized.                 |
| `progress`          | `Pubkey` | The progress account that was initialized.               |
| `pool`              | `Pubkey` | The pool account that was used to validate the pool.     |
| `position`          | `Pubkey` | The position account that was created.                   |
| `owner`             | `Pubkey` | The owner account that was used to sign the transaction. |
| `timestamp`         | `i64`    | The timestamp when the day was closed.                   |
| `total_distributed` | `u64`    | The total amount that was distributed.                   |
| `creator_payout`    | `u64`    | The creator payout that was processed.                   |
| `carry`             | `u64`    | The carryover from the previous day.                     |

```rust
#[event]
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub policy: Pubkey,
    pub progress: Pubkey,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
    pub total_distributed: u64,
    pub creator_payout: u64,
    pub carry: u64,
}
```
