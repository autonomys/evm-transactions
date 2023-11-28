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
        if transfer_amount <= gas.into() {
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

pub async fn circle_of_transfers(
    transaction_manager: TransactionManager,
    tx_mgrs: Vec<TransactionManager>,
    init_fund: U256,
    interval_second: usize,
    tx_count: usize,
) -> Result<()> {
    assert!(tx_mgrs.len() > 1);
    let first_sender = tx_mgrs[0].get_address();
    let last_receiver = tx_mgrs[tx_mgrs.len() - 1].clone();
    let transfer_pairs: Vec<_> = tx_mgrs
        .into_iter()
        .map_windows(|[x, y]| (x.clone(), y.get_address()))
        .chain(vec![(last_receiver, first_sender)])
        .collect();
    let total_pairs = transfer_pairs.len();
    let last_holder_index = transfer_pairs.len() - 1;

    // Send `holder_fund` to the first account
    let holder_fund = init_fund * 2;
    let tx = TransactionRequest::new()
        .to(first_sender)
        .value(holder_fund)
        .from(transaction_manager.get_address());
    transaction_manager.handle_transaction(tx).await?;

    let mut round = 0;
    let mut round_started_at = 0;
    let mut holder_index = 0;
    let mut round_ended = false;
    for iteration in 0..tx_count {
        let gas_price = transaction_manager.client.get_gas_price().await.ok();
        for (i, (sender, receiver)) in transfer_pairs.iter().enumerate() {
            let transfer_amount = if i == holder_index {
                holder_fund + U256::from(iteration)
            } else {
                U256::from(iteration)
            };
            let mut tx = TransactionRequest::new()
                .from(sender.get_address())
                .to(*receiver)
                .value(transfer_amount);

            // Always pay more gas to replace the previous tx with the same nonce
            if let Some(gp) = gas_price {
                tx = tx.gas_price(gp + U256::from(iteration));
            }

            if let Err(err) = sender.client.send_transaction(tx, None).await {
                log::info!(
                    "Error sending transaction for account #{}, err: {err:?}",
                    i + 1
                );
            }
        }

        // Sleep for `interval_second` to give some time for the tx propagate to other nodes
        // and maybe included in the bundle/block
        tokio::time::sleep(Duration::from_secs(interval_second as u64)).await;

        // Check who currently hold the `holder_fund` starting from the previous holder
        // and move to the next round if possible
        let check_balance_iter = transfer_pairs
            .iter()
            .enumerate()
            .cycle()
            .skip(holder_index)
            .take(total_pairs);
        for (i, (sender, _)) in check_balance_iter {
            let balance = transaction_manager
                .client
                .get_balance(sender.get_address(), None)
                .await
                .unwrap_or(0.into());
            if balance >= holder_fund {
                info!("Holder fund transfer to account #{}", i + 1);
                holder_index = i;

                // Start a new round if the `holder_fund` just transferred from the last account to the first
                if round_ended && holder_index == 0 {
                    round += 1;
                    let took = iteration - round_started_at;
                    round_started_at = iteration;
                    info!(
                        "Start round #{round} transfer, last round took {took} iterations, \
                        total run {iteration} iterations"
                    );
                }
                // Round ended when the `holder_fund` transferred to the last account
                round_ended = holder_index == last_holder_index;

                break;
            }
        }
    }

    Ok(())
}
