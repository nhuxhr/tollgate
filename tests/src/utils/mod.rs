use std::ops::Range;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use rand::seq::{IndexedRandom, SliceRandom};
use tollgate::constants::{POLICY_SEED, PROGRESS_SEED};

use crate::utils::svm::TestContext;

pub mod damm_v2;
pub mod streamflow;
pub mod svm;

/// Finds a program-derived address (PDA) for the given seeds and program ID.
///
/// # Arguments
/// * `seeds` - A slice of byte slices representing the seeds for the PDA
///
/// # Returns
/// A tuple containing the PDA (`Pubkey`) and its bump seed (`u8`)
pub fn find_program_address(seeds: &[&[u8]], program_id: Option<&Pubkey>) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id.unwrap_or(&tollgate::ID))
}

pub fn find_program_event_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"__event_authority"], program_id)
}

pub fn rand_investors_num(num_rng: Range<u32>) -> u32 {
    let mut rng = rand::rng();
    let mut nums: Vec<u32> = (num_rng).collect();
    nums.shuffle(&mut rng);
    *nums.choose(&mut rng).expect("")
}

pub fn log_policy_account(ctx: &TestContext, key: &str) {
    let token = ctx.tokens.get(key).expect("");
    let vault = token.vault.pubkey();
    let policy = find_program_address(&[POLICY_SEED, vault.as_ref()], None).0;
    let policy_acc = ctx.get_program_account::<tollgate::state::Policy>(&policy);
    println!("{:#?}", policy_acc);
}

pub fn log_progress_account(ctx: &TestContext, key: &str) {
    let token = ctx.tokens.get(key).expect("");
    let vault = token.vault.pubkey();
    let progress = find_program_address(&[PROGRESS_SEED, vault.as_ref()], None).0;
    let progress_acc = ctx.get_program_account::<tollgate::state::Progress>(&progress);
    println!("{:#?}", progress_acc);
}
