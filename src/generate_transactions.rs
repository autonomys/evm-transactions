use crate::{contract_calls::*, transaction_manager::TransactionManager, CHAIN_ID};
use ethers::prelude::*;
use eyre::Result;
use log::info;
use std::sync::Arc;
use std::time::Duration;

pub async fn generate_and_send_transfer(
    tx_manager: &TransactionManager,
    transfer_amount: &U256,
) -> Result<TransactionManager> {
    // Generate a new wallet for the recipient
    let recipient_wallet = Wallet::new(&mut rand::thread_rng()).with_chain_id(CHAIN_ID);
    let to = recipient_wallet.address();

    // Define the transfer
    let tx = TransactionRequest::new()
        .to(to)
        .value(transfer_amount)
        .from(tx_manager.get_address());

    info!(
        "Sending {:?} from {:?} to {:?}",
        transfer_amount,
        tx_manager.get_address(),
        to
    );
    tx_manager.handle_transaction(tx).await?;

    let provider = Arc::new(tx_manager.client.provider().clone()); //.clone();
    let recipient_tx_manager = TransactionManager::new(provider, &recipient_wallet);

    Ok(recipient_tx_manager)
}

pub async fn generate_and_send_set_array(
    tx_manager: TransactionManager,
    num_transactions: usize,
    load_contract_address: Address,
    count: U256,
) -> Result<()> {
    for i in 0..num_transactions {
        info!(
            "Transaction #{} for wallet {:?}",
            i + 1,
            tx_manager.get_address()
        );
        let tx = set_array_transaction(load_contract_address, count)?;
        tx_manager.handle_transaction(tx).await?;
    }

    Ok(())
}

pub async fn chain_of_transfers(
    transaction_manager: TransactionManager,
    num_transactions: usize,
    mut transfer_amount: U256,
) -> Result<()> {
    let gas = 1.2e13 as u64;
    let mut next_tx_manager = transaction_manager.clone();

    for i in 1..=num_transactions {
        if transfer_amount <= (1e14 as u128).into() {
            info!("Insufficient funds for transaction #{}", i);
            break;
        }
        info!(
            "Transaction #{} for wallet {:?}",
            i,
            next_tx_manager.get_address()
        );

        transfer_amount -= gas.into();
        next_tx_manager = generate_and_send_transfer(&next_tx_manager, &transfer_amount).await?;
    }
    Ok(())
}
