mod contract_calls;
mod generate_transactions;
mod transaction_manager;

use contract_calls::bulk_transfer_transaction;
use env_logger::Builder;
use ethers::prelude::*;
use eyre::{Report, Result};
use futures::future::join_all;
use generate_transactions::send_continuous_transactions;
use log::LevelFilter;
use std::env;
use std::sync::Arc;
use structopt::StructOpt;
use transaction_manager::TransactionManager;

pub const CHAIN_ID: u64 = 1002u64;
// Define a struct to hold the command-line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "EVM Transaction Generator")]
struct Opt {
    // The number of transactions to generate
    #[structopt(short, long)]
    tx_count: usize,

    // The number of accounts to use to generate transactions
    #[structopt(short, long)]
    num_accounts: usize,
}

struct EnvVars {
    funder_private_key: String,
    fund_contract_address: Address,
    rpc_url: String,
}
impl EnvVars {
    fn get_env_vars() -> eyre::Result<EnvVars> {
        let funder_private_key = env::var("FUNDER_PRIVATE_KEY").expect("PRIVATE_KEY must be set");
        let fund_contract_address: Address = env::var("FUNDING_CONTRACT_ADDRESS")
            .expect("FUND_CONTRACT_ADDRESS must be set")
            .parse()?;
        let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
        {
            Ok(EnvVars {
                funder_private_key,
                fund_contract_address,
                rpc_url,
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
        rpc_url,
    } = EnvVars::get_env_vars()?;

    // Parse command-line arguments
    let Opt {
        tx_count,
        num_accounts,
    } = Opt::from_args();

    let provider = Arc::new(Provider::<Http>::try_from(rpc_url).map_err(Report::msg)?);
    let funder_wallet: LocalWallet = funder_private_key.parse()?;
    let funder_wallet = funder_wallet.clone().with_chain_id(CHAIN_ID);
    let funder_tx_manager = TransactionManager::new(provider.clone(), &funder_wallet);

    let wallets = (0..num_accounts)
        .map(|_| Wallet::new(&mut rand::thread_rng()).with_chain_id(CHAIN_ID))
        .collect::<Vec<_>>();
    let addresses = wallets.iter().map(|w| w.address()).collect::<Vec<_>>();

    let tx = bulk_transfer_transaction(
        addresses,
        (100_000_000_000_000_000 as u128).into(),
        fund_contract_address,
    )
    .await?;
    funder_tx_manager.handle_transaction(tx).await?;

    // Transaction generation and sending
    let transactions = wallets
        .iter()
        .map(|w| {
            let tx_manager = TransactionManager::new(provider.clone(), &w);
            send_continuous_transactions(tx_manager.clone(), tx_count)
        })
        .collect::<Vec<_>>();

    let _results = join_all(transactions).await;
    Ok(())
}
