use crate::{contract_calls::set_array_transaction, transaction_manager::TransactionManager};
use ethers::prelude::*;
use eyre::Result;
use log::info;

pub enum TransactionType {
    Transfer,
    SetArray {
        contract_address: Address,
        count: U256,
    },
}

pub async fn send_continuous_transactions(
    transaction_manager: TransactionManager,
    num_transactions: usize,
    transaction_type: &TransactionType,
) -> Result<()> {
    for i in 0..num_transactions {
        info!(
            "Transaction #{} for wallet {}",
            i + 1,
            transaction_manager.get_address()
        );
        let _ = match transaction_type {
            TransactionType::Transfer => generate_and_send_transfer(&transaction_manager).await,
            TransactionType::SetArray {
                contract_address,
                count,
            } => {
                generate_and_send_set_array(
                    &transaction_manager,
                    contract_address.clone(),
                    count.clone(),
                )
                .await
            }
        };
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

async fn generate_and_send_set_array(
    tx_manager: &TransactionManager,
    load_contract_address: Address,
    count: U256,
) -> Result<()> {
    let tx = set_array_transaction(load_contract_address, count)?;
    tx_manager.handle_transaction(tx).await?;

    Ok(())
}
