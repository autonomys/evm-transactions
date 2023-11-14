use env_logger::Builder;
use ethers::prelude::*;
use eyre::{Report, Result};
use log::info;
use log::LevelFilter;
use std::env;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::time::{sleep, Duration};

mod transaction_manager;
use transaction_manager::TransactionManager;

// Define a struct to hold the command-line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "EVM Transaction Generator")]
struct Opt {
    // Sets the Ethereum node URL
    #[structopt(short, long)]
    node_url: String,

    // The number of transactions to generate
    #[structopt(short, long)]
    tx_count: usize,

    // The private key of the sender
    #[structopt(short, long)]
    private_key: String,
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

    // Parse command-line arguments
    let opt = Opt::from_args();

    let provider = Arc::new(Provider::<Http>::try_from(opt.node_url).map_err(Report::msg)?);
    let wallet: LocalWallet = opt.private_key.parse().map_err(Report::msg)?;
    let wallet = wallet.clone().with_chain_id(1002u64);

    let tx_manager = TransactionManager::new(provider.clone(), &wallet);

    // Transaction generation and sending
    for i in 0..opt.tx_count {
        info!("Transaction #{}", i + 1);
        generate_and_send_transaction(&tx_manager)
            .await
            .map_err(Report::msg)?;
        // sleep(Duration::from_millis(2500)).await;
    }

    Ok(())
}

// Assuming this function signature
async fn generate_and_send_transaction(tx_manager: &TransactionManager) -> Result<(), Report> {
    // Generate a new wallet for the recipient
    let recipient_wallet = Wallet::new(&mut rand::thread_rng());
    let to = recipient_wallet.address();

    // Define the transaction
    let tx = TransactionRequest::new()
        .to(to)
        .value(1e8 as u64)
        .from(tx_manager.get_address());

    tx_manager.handle_transaction(tx).await?;

    Ok(())
}
