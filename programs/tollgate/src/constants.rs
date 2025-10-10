use anchor_lang::prelude::*;

/// Seeds for PDA accounts

#[constant]
pub const POLICY_SEED: &[u8] = b"policy";

#[constant]
pub const PROGRESS_SEED: &[u8] = b"progress";

#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

#[constant]
pub const INVESTOR_FEE_POS_OWNER: &[u8] = b"investor_fee_pos_owner";

/// DAMM v2 constants
pub mod damm_v2_constants {
    pub mod seeds {
        pub const CONFIG_PREFIX: &[u8] = b"config";
        pub const CUSTOMIZABLE_POOL_PREFIX: &[u8] = b"cpool";
        pub const POOL_PREFIX: &[u8] = b"pool";
        pub const TOKEN_VAULT_PREFIX: &[u8] = b"token_vault";
        pub const POOL_AUTHORITY_PREFIX: &[u8] = b"pool_authority";
        pub const POSITION_PREFIX: &[u8] = b"position";
        pub const POSITION_NFT_ACCOUNT_PREFIX: &[u8] = b"position_nft_account";
        pub const TOKEN_BADGE_PREFIX: &[u8] = b"token_badge";
        pub const REWARD_VAULT_PREFIX: &[u8] = b"reward_vault";
        pub const CLAIM_FEE_OPERATOR_PREFIX: &[u8] = b"cf_operator";
    }

    pub mod pool_authority {
        use anchor_lang::solana_program::pubkey::Pubkey;
        use const_crypto::ed25519;

        use super::*;

        const POOL_AUTHORITY_AND_BUMP: ([u8; 32], u8) = ed25519::derive_program_address(
            &[seeds::POOL_AUTHORITY_PREFIX],
            &damm_v2::ID_CONST.to_bytes(),
        );

        pub const ID: Pubkey = Pubkey::new_from_array(POOL_AUTHORITY_AND_BUMP.0);
        pub const BUMP: u8 = POOL_AUTHORITY_AND_BUMP.1;
    }
}

/// Time constants

#[constant]
pub const TWENTY_FOUR_HOURS: i64 = 86_400; // in seconds

/// Basis points constants

#[constant]
pub const MAX_BPS: u16 = 10_000; // 100%
