use std::ops::Range;

use anchor_client::solana_sdk::pubkey::Pubkey;
use rand::seq::{IndexedRandom, SliceRandom};

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
