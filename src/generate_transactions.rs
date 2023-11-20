use crate::{contract_calls::set_array_transaction, transaction_manager::TransactionManager};
use ethers::prelude::*;
use eyre::Result;
use log::info;
use std::sync::Arc;

pub enum TransactionType {
    Transfer {
        hierarchical_tx_depth: u64,
    },
    SetArray {
        contract_address: Address,
        count: U256,
    },
}

pub async fn send_continuous_transactions(
    provider: Arc<Provider<Http>>,
    transaction_manager: TransactionManager,
    num_transactions: usize,
    transaction_type: &TransactionType,
) -> Result<()> {
    for i in 0..num_transactions {
        info!(
            "Transaction #{} for wallet {:?}",
            i + 1,
            transaction_manager.get_address()
        );
        let _ = match transaction_type {
            TransactionType::Transfer {
                hierarchical_tx_depth,
            } => {
                generate_and_send_transfer(
                    provider.clone(),
                    transaction_manager.clone(),
                    *hierarchical_tx_depth,
                )
                .await
            }
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

async fn generate_and_send_transfer(
    provider: Arc<Provider<Http>>,
    mut tx_manager: TransactionManager,
    level: u64,
) -> Result<()> {
    for i in (1..=level).rev() {
        // Generate a new wallet for the recipient
        let recipient_wallet = Wallet::new(&mut rand::thread_rng());
        let to = recipient_wallet.address();

        // Transfer `total - 1e12` to the recipient and leave 1e12 as tx fee
        let tx = TransactionRequest::new()
            .to(to)
            .value(1e12 as u64 * i)
            .from(tx_manager.get_address());

        tx_manager.handle_transaction(tx).await?;

        // Use the `recipient_wallet` as the next sender
        tx_manager = TransactionManager::new(provider.clone(), &recipient_wallet);
    }

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
