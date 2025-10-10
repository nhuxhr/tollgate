use anchor_lang::prelude::*;

use crate::{constants::TWENTY_FOUR_HOURS, error::TollgateError};

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq)]
pub enum DayState {
    New,    // New day, no distributions yet
    Same,   // Same day, distributions ongoing
    Closed, // Closed for the day, no more distributions
}

#[account]
#[derive(InitSpace)]
pub struct Progress {
    pub vault: Pubkey,             // Associated vault
    pub last_distribution_ts: i64, // Timestamp of last distribution
    pub daily_spent: u64,          // Amount spent in current day
    pub carry: u64,                // Carryover from prev day
    pub cursor: u32,               // Pagination index in remaining_accounts
    pub day_state: DayState,       // State of the current day
    pub bump: u8,                  // PDA bump
}

impl Progress {
    pub const SPACE: usize = Self::DISCRIMINATOR.len() + Self::INIT_SPACE;

    /// Initializes the Progress account.
    pub fn initialize(&mut self, vault: Pubkey, bump: u8) -> Result<()> {
        // assert progress is not already initialized
        require!(
            self.last_distribution_ts == 0,
            TollgateError::ProgressAlreadyInitialized
        );

        self.vault = vault;
        self.last_distribution_ts = 0;
        self.daily_spent = 0;
        self.carry = 0;
        self.cursor = 0;
        self.day_state = DayState::Closed;
        self.bump = bump;

        Ok(())
    }

    /// Checks whether a new day has started based on the given timestamp.
    pub fn is_new_day(&self, now_ts: i64) -> bool {
        if self.last_distribution_ts == 0 {
            // Never started a day before
            return true;
        }
        // 24 hours (86,400 seconds) since last distribution.
        now_ts - self.last_distribution_ts >= TWENTY_FOUR_HOURS
    }

    /// Checks whether it is the same day based on the given timestamp.
    pub fn is_same_day(&self, now_ts: i64) -> bool {
        // Less than 24 hours (86,400 seconds) since last distribution.
        !self.is_new_day(now_ts)
    }

    /// Starts a new day session if allowed.
    pub fn start_new_day(&mut self, now_ts: i64) -> Result<()> {
        require!(self.is_new_day(now_ts), TollgateError::CannotStartNewDay);

        self.day_state = DayState::New;
        self.last_distribution_ts = now_ts;
        self.daily_spent = 0;
        self.cursor = 0;

        Ok(())
    }

    /// Switches active day from New to Same.
    pub fn continue_same_day(&mut self) -> Result<()> {
        require!(
            self.day_state == DayState::New,
            TollgateError::CannotContinueSameDay
        );

        self.day_state = DayState::Same;

        Ok(())
    }

    /// Closes the current day session.
    pub fn close_day(&mut self) -> Result<()> {
        require!(
            self.day_state == DayState::New || self.day_state == DayState::Same,
            TollgateError::CannotCloseDay
        );

        self.day_state = DayState::Closed;

        Ok(())
    }
}
