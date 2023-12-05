#![feature(iter_map_windows)]

mod contract_calls;
mod generate_transactions;
mod transaction_manager;

use contract_calls::*;
use env_logger::Builder;
use ethers::prelude::*;
use eyre::{Report, Result};
use futures::future::try_join_all;
use generate_transactions::*;
use log::LevelFilter;
use std::env;
use std::sync::Arc;
use structopt::StructOpt;
use transaction_manager::TransactionManager;

// Define a struct to hold the command-line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "EVM Transaction Generator")]
struct Opt {
    // The number of accounts to use to generate transactions
    #[structopt(short, long)]
    num_accounts: usize,

    // The amount of funding to send to each account
    #[structopt(short, long)]
    funding_amount_tssc: f64,

    // The number of transactions to generate
    #[structopt(short, long)]
    tx_count: usize,

    #[structopt(subcommand)]
    tx_type: TransactionType,
}

#[derive(StructOpt, Debug)]
enum TransactionType {
    /// Generate transaction with given weight/size
    SetArray {
        /// Measurement of how heavy a transaction should be, values between 1-2000 are appropriate
        set_array_count: U256,
    },
    /// Generate chain of transfer transaction, i.e. A->B, B->C, ..
    ChainTransfer,
    /// Generate circle of transfer transaction, i.e. A->B, B->C, C->A
    CircleTransfer {
        /// Interval between transactions
        interval_second: usize,
    },
}

struct EnvVars {
    funder_private_key: String,
    fund_contract_address: Address,
    load_contract_address: Address,
    rpc_url: String,
    num_confirmations: usize,
}
impl EnvVars {
    fn get_env_vars() -> eyre::Result<EnvVars> {
        let funder_private_key = env::var("FUNDER_PRIVATE_KEY").expect("PRIVATE_KEY must be set");
        let fund_contract_address: Address = env::var("FUNDING_CONTRACT_ADDRESS")
            .expect("FUND_CONTRACT_ADDRESS must be set")
            .parse()?;
        let load_contract_address: Address = env::var("LOAD_CONTRACT_ADDRESS")
            .expect("LOAD_CONTRACT_ADDRESS must be set")
            .parse()?;
        let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
        let num_confirmations: usize = env::var("NUM_CONFIRMATIONS")
            .unwrap_or("3".to_owned())
            .parse()?;
        {
            Ok(EnvVars {
                funder_private_key,
                fund_contract_address,
                load_contract_address,
                rpc_url,
                num_confirmations,
            })
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    // Initialize env_logger
    let mut builder = Builder::from_default_env();

    // Set the default filter level to `info` if no environment variable is set
    if env::var("RUST_LOG").is_err() {
        builder.filter(None, LevelFilter::Info);
    }

    builder.init();

    dotenv::from_path(".env")?;
    let EnvVars {
        funder_private_key,
        fund_contract_address,
        load_contract_address,
        rpc_url,
        num_confirmations,
    } = EnvVars::get_env_vars()?;

    // Parse command-line arguments
    let Opt {
        num_accounts,
        funding_amount_tssc,
        tx_count,
        tx_type,
    } = Opt::from_args();

    let provider = Arc::new(Provider::<Http>::try_from(rpc_url).map_err(Report::msg)?);
    let chain_id: u64 = provider.get_chainid().await?.as_u64();

    let funder_tx_manager = {
        let funder_wallet = funder_private_key
            .parse::<LocalWallet>()?
            .with_chain_id(chain_id);
        TransactionManager::new(provider.clone(), &funder_wallet, num_confirmations)
    };
    let funding_amount: U256 = ((funding_amount_tssc * 1e18) as u128).into();
    let acc_tx_mgrs = (0..num_accounts)
        .map(|_| Wallet::new(&mut rand::thread_rng()).with_chain_id(chain_id))
        .map(|w| TransactionManager::new(provider.clone(), &w, num_confirmations))
        .collect::<Vec<_>>();

    // Initial fund for accounts
    let initial_fund_tx = {
        let addresses = acc_tx_mgrs
            .iter()
            .map(|w| w.get_address())
            .collect::<Vec<_>>();
        bulk_transfer_transaction(addresses, funding_amount, fund_contract_address)?
    };
    funder_tx_manager
        .handle_transaction(initial_fund_tx)
        .await?;

    match tx_type {
        TransactionType::SetArray { set_array_count } => {
            let transactions = acc_tx_mgrs
                .into_iter()
                .map(|tx_mgr| {
                    generate_and_send_set_array(
                        tx_mgr,
                        tx_count,
                        load_contract_address,
                        set_array_count,
                    )
                })
                .collect::<Vec<_>>();

            try_join_all(transactions).await?;
        }
        TransactionType::ChainTransfer => {
            let transactions = acc_tx_mgrs
                .into_iter()
                .map(|tx_mgr| {
                    chain_of_transfers(tx_mgr, tx_count, funding_amount, num_confirmations)
                })
                .collect::<Vec<_>>();

            try_join_all(transactions).await?;
        }
        TransactionType::CircleTransfer { interval_second } => {
            circle_of_transfers(
                funder_tx_manager,
                acc_tx_mgrs,
                funding_amount,
                interval_second,
                tx_count,
            )
            .await?;
        }
    }

    Ok(())
}
