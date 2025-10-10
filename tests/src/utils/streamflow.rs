use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_sdk::{instruction::Instruction, system_program, sysvar::rent},
};
use anchor_spl::{
    associated_token::{get_associated_token_address, spl_associated_token_account},
    token::spl_token,
};
use solana_pubkey::Pubkey;

pub fn get_create_stream_ix_accs(
    sender: Pubkey,
    recipient: Pubkey,
    metadata: Pubkey,
    mint: Pubkey,
) -> streamflow_sdk::accounts::Create {
    let streamflow_treasury = Pubkey::from_str_const(streamflow_sdk::state::STRM_TREASURY);
    let streamflow_treasury_tokens = get_associated_token_address(&streamflow_treasury, &mint);

    streamflow_sdk::accounts::Create {
        sender,
        sender_tokens: get_associated_token_address(&sender, &mint),
        recipient,
        recipient_tokens: get_associated_token_address(&recipient, &mint),
        metadata,
        escrow_tokens: streamflow_sdk::state::find_escrow_account(
            metadata.as_ref(),
            &streamflow_sdk::ID,
        )
        .0,
        streamflow_treasury,
        streamflow_treasury_tokens,
        withdrawor: Pubkey::from_str_const(streamflow_sdk::state::WITHDRAWOR_ADDRESS),
        partner: streamflow_treasury,
        partner_tokens: streamflow_treasury_tokens,
        mint,
        fee_oracle: Pubkey::from_str_const(streamflow_sdk::state::FEE_ORACLE_ADDRESS),
        rent: rent::ID,
        timelock_program: streamflow_sdk::ID,
        token_program: spl_token::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
    }
}

pub fn create_stream_ix(
    accounts: impl ToAccountMetas,
    args: streamflow_sdk::instruction::Create,
) -> Instruction {
    Instruction::new_with_bytes(
        streamflow_sdk::ID,
        &args.data(),
        accounts.to_account_metas(None),
    )
}

pub fn get_withdraw_stream_ix_accs(
    authority: Pubkey,
    recipient: Pubkey,
    metadata: Pubkey,
    mint: Pubkey,
) -> streamflow_sdk::accounts::Withdraw {
    let streamflow_treasury = Pubkey::from_str_const(streamflow_sdk::state::STRM_TREASURY);
    let streamflow_treasury_tokens = get_associated_token_address(&streamflow_treasury, &mint);

    streamflow_sdk::accounts::Withdraw {
        authority,
        recipient,
        recipient_tokens: get_associated_token_address(&recipient, &mint),
        metadata,
        escrow_tokens: streamflow_sdk::state::find_escrow_account(
            metadata.as_ref(),
            &streamflow_sdk::ID,
        )
        .0,
        streamflow_treasury,
        streamflow_treasury_tokens,
        partner: streamflow_treasury,
        partner_tokens: streamflow_treasury_tokens,
        mint,
        token_program: spl_token::ID,
    }
}

pub fn withdraw_stream_ix(
    accounts: impl ToAccountMetas,
    args: streamflow_sdk::instruction::Withdraw,
) -> Instruction {
    Instruction::new_with_bytes(
        streamflow_sdk::ID,
        &args.data(),
        accounts.to_account_metas(None),
    )
}
