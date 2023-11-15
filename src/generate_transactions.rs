use crate::transaction_manager::TransactionManager;
use ethers::{
    core::rand,
    signers::{Signer, Wallet},
    types::TransactionRequest,
};
use eyre::Result;
use log::info;

pub async fn send_continuous_transactions(
    transaction_manager: TransactionManager,
    num_transactions: usize,
) -> Result<()> {
    let mut futures = Vec::with_capacity(num_transactions);

    for i in 0..num_transactions {
        info!(
            "Transaction #{} for wallet {}",
            i + 1,
            transaction_manager.get_address()
        );
        let tx_future = generate_and_send_transfer(&transaction_manager).await;
        futures.push(Box::pin(tx_future));
    }

    Ok(())
}

async fn generate_and_send_transfer(tx_manager: &TransactionManager) -> Result<()> {
    // Generate a new wallet for the recipient
    let recipient_wallet = Wallet::new(&mut rand::thread_rng());
    let to = recipient_wallet.address();

    // Define the transfer
    let tx = TransactionRequest::new()
        .to(to)
        .value(1e6 as u64)
        .from(tx_manager.get_address());

    tx_manager.handle_transaction(tx).await?;

    Ok(())
}
