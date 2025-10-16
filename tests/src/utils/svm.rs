use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use anchor_client::{
    anchor_lang::AccountDeserialize,
    solana_sdk::{
        account::Account,
        instruction::{Instruction, InstructionError},
        native_token::LAMPORTS_PER_SOL,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
        signer::Signer,
        system_instruction::{self, SystemError},
        system_program,
        transaction::{Transaction, TransactionError},
    },
};
use anchor_spl::{
    associated_token::{get_associated_token_address, spl_associated_token_account},
    token::spl_token::{
        self,
        instruction::{initialize_mint, mint_to},
        state::Mint,
    },
    token_2022::spl_token_2022,
};
use base64::{engine::general_purpose, Engine};
use lazy_static::lazy_static;
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use solana_clock::Clock;
use tollgate::error::TollgateError;

use crate::constants::{SOL_MINT, USDC_MINT};

pub type TransactionResult = Result<TransactionMetadata, Box<FailedTransactionMetadata>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAccountData {
    data: Vec<String>,
    executable: bool,
    lamports: u64,
    owner: String,
    rent_epoch: u64,
    space: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAccountInfo {
    account: JsonAccountData,
    pubkey: String,
}

#[derive(Debug, Clone)]
pub struct Investor {
    pub key: Arc<Keypair>,
    pub stream: Arc<Keypair>,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub creator: Arc<Keypair>,
    pub base_mint: Arc<Keypair>,
    pub quote_mint: Pubkey,
    pub pool_config: Pubkey,
    pub pos_mints: HashMap<String, Arc<Keypair>>,
    pub vault: Arc<Keypair>,
    pub investors: Vec<Investor>,
}

// Helper function to load account from dumped JSON
fn load_account_from_json(path: &str) -> JsonAccountInfo {
    let json_str = fs::read_to_string(path).expect("Failed to read JSON file");
    from_str(&json_str).unwrap()
}

lazy_static! {
    static ref PAYER: Keypair = {
        let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
        read_keypair_file(&anchor_wallet).unwrap()
    };
    static ref SVM: Mutex<LiteSVM> = {
        let mut svm = LiteSVM::new();

        svm.add_program(
            spl_token::ID,
            include_bytes!("../../../.tollgate/programs/spl_token.so"),
        );

        svm.add_program(
            spl_token_2022::ID,
            include_bytes!("../../../.tollgate/programs/spl_token_2022.so"),
        );

        svm.add_program(
            spl_associated_token_account::ID,
            include_bytes!("../../../.tollgate/programs/spl_ata.so"),
        );

        svm.add_program(
            damm_v2::ID,
            include_bytes!("../../../.tollgate/programs/damm_v2.so"),
        );

        svm.add_program(
            streamflow_sdk::ID,
            include_bytes!("../../../.tollgate/programs/streamflow.so"),
        );

        svm.add_program(
            tollgate::ID,
            include_bytes!("../../../target/deploy/tollgate.so"),
        );

        let wsol_mint_key = SOL_MINT;
        let wsol_mint_struct = Mint {
            mint_authority: COption::None,
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut wsol_mint_bytes = [0u8; Mint::LEN];
        Mint::pack(wsol_mint_struct, &mut wsol_mint_bytes).unwrap();
        svm.set_account(
            wsol_mint_key,
            Account {
                lamports: 1_000_000_000,
                data: wsol_mint_bytes.to_vec(),
                owner: spl_token::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        let usdc_mint_key = USDC_MINT;
        let usdc_mint_struct = Mint {
            mint_authority: COption::Some(PAYER.pubkey()),
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut usdc_mint_bytes = [0u8; Mint::LEN];
        Mint::pack(usdc_mint_struct, &mut usdc_mint_bytes).unwrap();
        svm.set_account(
            usdc_mint_key,
            Account {
                lamports: 1_000_000_000,
                data: usdc_mint_bytes.to_vec(),
                owner: spl_token::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        svm.set_account(
            Pubkey::from_str_const(streamflow_sdk::state::STRM_TREASURY),
            Account {
                lamports: 4142925334,
                data: vec![],
                owner: system_program::ID,
                executable: false,
                rent_epoch: 18446744073709551615,
            },
        )
        .unwrap();

        let test_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pool_config_accs_dir = test_dir.join("../.tollgate/accounts/pool_config");
        for entry in pool_config_accs_dir
            .read_dir()
            .expect("Failed to read directory")
        {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
                let acc = load_account_from_json(
                    path.to_str().expect("Failed to convert path to string"),
                );
                svm.set_account(
                    Pubkey::from_str_const(&acc.pubkey),
                    Account {
                        lamports: acc.account.lamports,
                        data: general_purpose::STANDARD
                            .decode(&acc.account.data[0])
                            .expect("Failed to decode base64"),
                        owner: Pubkey::from_str_const(&acc.account.owner),
                        executable: acc.account.executable,
                        rent_epoch: acc.account.rent_epoch,
                    },
                )
                .unwrap();
            }
        }

        svm.airdrop(&PAYER.pubkey(), 100_000 * LAMPORTS_PER_SOL)
            .unwrap();

        Mutex::new(svm)
    };
    static ref TOKENS: Mutex<HashMap<String, Token>> = Mutex::new(HashMap::new());
}

pub fn get_payer() -> &'static Keypair {
    &PAYER
}

pub struct TestContext {
    pub payer: Arc<Keypair>,
    pub svm: MutexGuard<'static, LiteSVM>,
    pub tokens: MutexGuard<'static, HashMap<String, Token>>,
}

impl Default for TestContext {
    fn default() -> Self {
        Self {
            payer: {
                let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
                Arc::new(read_keypair_file(&anchor_wallet).unwrap())
            },
            svm: SVM.lock().unwrap(),
            tokens: TOKENS.lock().unwrap(),
        }
    }
}

impl TestContext {
    pub fn airdrop(
        &mut self,
        address: &Pubkey,
        sol: u64,
    ) -> Result<TransactionMetadata, Box<FailedTransactionMetadata>> {
        self.svm
            .airdrop(address, sol * LAMPORTS_PER_SOL)
            .map_err(Box::new)
    }

    pub fn time_travel_duration(&mut self, secs: u64) -> i64 {
        let mut clock = self.svm.get_sysvar::<Clock>();
        let duration = Duration::from_secs(secs);
        clock.unix_timestamp += duration.as_secs() as i64;
        clock.unix_timestamp
    }

    pub fn time_travel_by_secs(&mut self, secs: u64) -> Clock {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.unix_timestamp = self.time_travel_duration(secs);
        self.svm.set_sysvar::<Clock>(&clock);
        clock
    }

    pub fn time_travel_to(&mut self, timestamp: i64) -> Clock {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.unix_timestamp = timestamp;
        self.svm.set_sysvar::<Clock>(&clock);
        clock
    }

    pub fn get_program_account<T: AccountDeserialize>(&self, address: &Pubkey) -> T {
        let account = self.svm.get_account(address).unwrap();
        let mut data = account.data.as_slice();
        (T::try_deserialize(&mut data)).unwrap()
    }

    pub fn send_transaction(
        &mut self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signers: &[&Keypair],
    ) -> TransactionResult {
        // Create and sign transaction
        let mut transaction = Transaction::new_with_payer(instructions, payer);
        transaction.sign(signers, self.svm.latest_blockhash());

        // Process transaction
        self.svm.send_transaction(transaction).map_err(Box::new)
    }

    pub fn create_spl_token(
        &mut self,
        creator: Option<&Keypair>,
        mint: Option<Keypair>,
        amount: u64,
    ) -> Keypair {
        let creator = creator.unwrap_or(get_payer());
        let mint = mint.unwrap_or(Keypair::new());

        // Calculate rent-exempt minimum balance for mint account
        let rent = self.svm.minimum_balance_for_rent_exemption(Mint::LEN);

        // Get the associated token address for the payer and the new mint
        let ata = get_associated_token_address(&creator.pubkey(), &mint.pubkey());

        // Prepare instructions
        let instructions = vec![
            // Create mint account
            system_instruction::create_account(
                &creator.pubkey(),
                &mint.pubkey(),
                rent,
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            // Initialize mint
            initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &creator.pubkey(), // Mint authority
                None,              // No freeze authority
                6,                 // Decimals
            )
            .expect("Failed to create initialize mint instruction"),
            // Create associated token account
            spl_associated_token_account::instruction::create_associated_token_account(
                &creator.pubkey(),
                &creator.pubkey(),
                &mint.pubkey(),
                &spl_token::id(),
            ),
            // Mint tokens to ATA
            mint_to(
                &spl_token::id(),
                &mint.pubkey(),
                &ata,
                &creator.pubkey(),
                &[],
                amount,
            )
            .expect("Failed to create mint to instruction"),
        ];

        // Process transaction
        self.send_transaction(&instructions, Some(&creator.pubkey()), &[creator, &mint])
            .expect("Failed to process transaction");

        mint // Return the mint's keypair
    }
}

pub fn get_ix_err(err: TollgateError) -> InstructionError {
    InstructionError::Custom(6000 + err as u32)
}

pub fn demand_logs_contain(expected: &str, result: &TransactionResult) {
    let logs = match &result {
        Ok(meta) => &meta.logs,
        Err(meta) => &meta.meta.logs,
    };

    if logs.iter().any(|log| log.contains(expected)) {
        return;
    }

    panic!(
        "Expected {:?} among {} log entries: {}",
        expected,
        logs.len(),
        logs.iter()
            .enumerate()
            .map(|(i, log)| format!("[{}]: {}", i, log))
            .collect::<Vec<_>>()
            .join(", ")
    );
}
pub fn demand_instruction_error(expected_error: InstructionError, result: &TransactionResult) {
    let Err(e) = result else {
        panic!("Expected {} but transaction succeeded", expected_error);
    };

    let TransactionError::InstructionError(_, observed_error) = &e.err else {
        panic!("Expected {} but got: {}", expected_error, e.err);
    };

    if *observed_error != expected_error {
        panic!("Expected {} but got {}", expected_error, observed_error);
    }
}

pub fn demand_instruction_one_of_errors(
    expected_errors: Vec<InstructionError>,
    result: &TransactionResult,
) {
    let Err(e) = result else {
        panic!(
            "Expected one of {:?} but transaction succeeded",
            expected_errors
        );
    };

    let TransactionError::InstructionError(_, observed_error) = &e.err else {
        panic!("Expected one of {:?} but got: {}", expected_errors, e.err);
    };

    if !expected_errors.contains(observed_error) {
        panic!(
            "Expected one of {:?} but got {}",
            expected_errors, observed_error
        );
    }
}

pub fn demand_transaction_error(expected: TransactionError, result: &TransactionResult) {
    let Err(e) = result else {
        panic!("Expected {} but transaction succeeded", expected);
    };

    if e.err != expected {
        panic!("Expected {} but got {}", expected, e.err);
    }
}

pub fn demand_system_error(expected_error: SystemError, result: &TransactionResult) {
    let Err(e) = &result else {
        panic!("Expected {} but transaction succeeded", expected_error);
    };

    let TransactionError::InstructionError(_, InstructionError::Custom(observed_code)) = &e.err
    else {
        panic!("Expected {} but got: {}", expected_error, e.err);
    };

    let Some(observed_error) = SystemError::from_u64(*observed_code as u64) else {
        panic!(
            "Expected {} but got invalid code {}",
            expected_error, observed_code
        );
    };

    if observed_error != expected_error {
        panic!("Expected {} but got: {}", expected_error, observed_error);
    }
}
