use env_logger::Builder;
use ethers::prelude::*;
use log::LevelFilter;
use log::{error, info};
use std::env;
use std::error::Error;
use std::sync::Arc;
use structopt::StructOpt;

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

    #[structopt(short, long)]
    private_key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize env_logger
    let mut builder = Builder::from_default_env();

    // Set the default filter level to `info` if no environment variable is set
    if env::var("RUST_LOG").is_err() {
        builder.filter(None, LevelFilter::Info);
    }

    builder.init();
    // Parse command-line arguments
    let opt = Opt::from_args();
    // Start an Actix system

    // Setting up Ethereum client
    let provider = Arc::new(Provider::<Http>::try_from(opt.node_url)?);
    let wallet: LocalWallet = opt.private_key.parse()?;
    let wallet = wallet.clone().with_chain_id(1002u64);

    // Transaction generation and sending
    for i in 0..opt.tx_count {
        info!("Transaction #{}", i + 1);
        generate_and_send_transaction(&provider, &wallet).await?;
    }

    Ok(())
}

// Assuming this function signature
async fn generate_and_send_transaction(
    provider: &Provider<Http>,
    wallet: &LocalWallet,
) -> Result<(), Box<dyn Error>> {
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    // Generate a new wallet for the recipient
    let recipient_wallet = Wallet::new(&mut rand::thread_rng());
    let to = recipient_wallet.address();

    // Define the transaction
    let tx = TransactionRequest::new()
        .to(to)
        .value(1e8 as u64) // Example amount
        .from(wallet.address());

    match client.send_transaction(tx, None).await {
        Ok(pending_tx) => {
            info!(
                "Transaction {:?} sent. Waiting for confirmation...",
                *pending_tx
            );

            let _receipt = pending_tx.confirmations(3).await?;
            info!("Transaction confirmed");
        }
        Err(e) => {
            error!("Error sending transaction: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
