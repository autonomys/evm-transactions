use std::sync::Arc;

use crate::{contract_calls::*, transaction_manager::TransactionManager, CHAIN_ID};
use ethers::prelude::*;
use eyre::Result;
use log::info;

pub enum TransactionType {
    Transfer(U256),
    SetArray {
        contract_address: Address,
        count: U256,
    },
    BulkTransfer {
        to_addresses: Vec<Address>,
        funding_amount: U256,
        contract_address: Address,
    },
}

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

async fn generate_and_send_set_array(
    tx_manager: &TransactionManager,
    load_contract_address: Address,
    count: U256,
) -> Result<()> {
    let tx = set_array_transaction(load_contract_address, count)?;
    tx_manager.handle_transaction(tx).await?;

    Ok(())
}

async fn generate_and_send_bulk_transfer(
    transaction_manager: &TransactionManager,
    to_addresses: Vec<Address>,
    funding_amount: U256,
    contract_address: Address,
) -> Result<()> {
    let tx = bulk_transfer_transaction(to_addresses, funding_amount, contract_address)?;
    transaction_manager.handle_transaction(tx).await?;

    Ok(())
}

pub async fn send_continuous_transactions(
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
            TransactionType::Transfer(amount) => {
                generate_and_send_transfer(&transaction_manager, amount).await?;
                Ok(())
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
            TransactionType::BulkTransfer {
                to_addresses,
                funding_amount,
                contract_address,
            } => {
                generate_and_send_bulk_transfer(
                    &transaction_manager,
                    to_addresses.clone(),
                    funding_amount.clone(),
                    contract_address.clone(),
                )
                .await
            }
        };
    }

    Ok(())
}

pub async fn chain_of_transfers(
    transaction_manager: TransactionManager,
    num_transactions: usize,
    amount: U256,
) -> Result<()> {
    let gas = 1.2e13 as u64;
    let mut next_tx_manager = transaction_manager.clone();

    for i in 0..num_transactions {
        let transfer_amount: U256 = amount - ((i + 1) as u64 * gas);
        if transfer_amount <= (1e14 as u128).into() {
            info!("Insufficient funds for transaction #{}", i + 1);
            break;
        }
        info!(
            "Transaction #{} for wallet {:?}",
            i + 1,
            next_tx_manager.get_address()
        );

        next_tx_manager = generate_and_send_transfer(&next_tx_manager, &transfer_amount).await?;
    }
    Ok(())
}
