use crate::{
    block_monitor::domain_block_monitor, contract_calls::*, transaction_manager::TransactionManager,
};
use ethers::prelude::*;
use ethers::signers::Signer;
use ethers_providers::Ws;
use eyre::Result;
use futures::prelude::future::join_all;
use log::info;
use rand::{thread_rng, Rng};
use std::mem;
use std::sync::Arc;
use std::time::Duration;

pub async fn generate_and_send_transfer(
    tx_manager: &TransactionManager,
    transfer_amount: &U256,
    num_confirmations: usize,
) -> Result<TransactionManager> {
    // Generate a new wallet for the recipient
    let recipient_wallet = Wallet::new(&mut rand::thread_rng()).with_chain_id(tx_manager.chain_id);
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
    let recipient_tx_manager =
        TransactionManager::new(provider, &recipient_wallet, num_confirmations);

    Ok(recipient_tx_manager)
}

pub async fn generate_and_send_set_array(
    tx_manager: TransactionManager,
    num_transactions: usize,
    load_contract_address: Address,
    count: U256,
) -> Result<()> {
    let random_value: U256 = thread_rng().gen_range(1..150).into();
    let count = count.checked_add(random_value).unwrap_or(count);
    let tx = set_array_transaction(load_contract_address, count)?;
    tx_manager.handle_transaction(tx).await?;
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
    num_confirmations: usize,
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
        next_tx_manager =
            generate_and_send_transfer(&next_tx_manager, &transfer_amount, num_confirmations)
                .await?;
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

const TX_PER_BATCH: u32 = 50;

async fn batch_send_fund(
    tx_manager: TransactionManager,
    fund_contract_address: Address,
    fund_per_account: U256,
    from: u32,
    count: u32,
    tx_per_batch: u32,
) -> Result<()> {
    let mut addresses = vec![];
    for i in 0..count {
        addresses.push(derive_wallet(from + i).address());
        if (i != 0 && i % tx_per_batch == 0) || (i == count - 1) {
            let tx = bulk_transfer_transaction(
                mem::take(&mut addresses),
                fund_per_account,
                fund_contract_address,
            )?
            .from(tx_manager.get_address())
            .chain_id(tx_manager.chain_id);

            tx_manager
                .client
                .send_transaction(tx.clone(), None)
                .await?
                .confirmations(1)
                .await?;
        }
    }
    Ok(())
}

async fn submit_transfer(
    provider: Arc<Provider<Ws>>,
    chain_id: u64,
    fund_per_account: U256,
    tx_count: usize,
) -> Result<()> {
    let mut submitted_transfer = 0;
    for idx in 0..tx_count {
        let from = derive_wallet(idx as u32).with_chain_id(chain_id);
        let to = derive_wallet(u32::MAX - idx as u32).address();
        let tx = TransactionRequest::new()
            .to(to)
            .value(fund_per_account / 5)
            .from(from.address());
        if SignerMiddleware::new(provider.clone(), from)
            .send_transaction(tx, None)
            .await
            .is_err()
        {
            continue;
        }
        submitted_transfer += 1;

        if idx % 100 == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    info!(
        "All transfer submitted, total_transfer {tx_count:?}, submitted_transfer {submitted_transfer:?}",
    );
    Ok(())
}

pub async fn concurrent_transfers(
    provider: Arc<Provider<Ws>>,
    tx_mgrs: Vec<TransactionManager>,
    chain_id: u64,
    fund_contract_address: Address,
    funding_amount: U256,
    tx_count: usize,
    domain_id: u32,
    consensus_url: String,
    domain_url: String,
) -> Result<()> {
    // Pre fund
    let concurrencies = tx_mgrs.len();
    let tx_per_mgr = tx_count / concurrencies;
    let fund_per_account = funding_amount / tx_per_mgr / 2;

    let mut batch_send_funds = vec![];
    for (i, tx_mgr) in tx_mgrs.into_iter().enumerate() {
        let fut = batch_send_fund(
            tx_mgr,
            fund_contract_address,
            fund_per_account,
            (i * tx_per_mgr) as u32,
            tx_per_mgr as u32,
            concurrencies as u32,
        );
        batch_send_funds.push(fut);
    }
    for res in join_all(batch_send_funds).await {
        if let Err(err) = res {
            info!("Batch transfer failed, err {err:?}");
            return Err(err.into());
        }
    }
    info!("Batch pre-fund finish, start concurrent transfer");

    let domain_block_monitor_fut =
        domain_block_monitor(domain_id, consensus_url, domain_url, tx_count as u64);
    let submit_transfer_fut =
        submit_transfer(provider.clone(), chain_id, fund_per_account, tx_count);

    let (domain_block_monitor_res, submit_transfer_res) =
        tokio::join!(domain_block_monitor_fut, submit_transfer_fut);
    if let Err(err) = domain_block_monitor_res {
        info!("Domain block monitor failed, err {err:?}");
    }
    if let Err(err) = submit_transfer_res {
        info!("Submit transfer failed, err {err:?}");
    }

    Ok(())
}

use ethers::core::k256::SecretKey;
use ethers::signers::LocalWallet;

fn derive_wallet(seed: u32) -> LocalWallet {
    let key = {
        let mut k = [1u8; 32];
        (k[..4]).copy_from_slice(&seed.to_be_bytes()[..]);
        k
    };
    let sk = SecretKey::from_slice(&key[..]).expect("Must success");
    LocalWallet::from_bytes(sk.to_bytes().as_slice()).expect("Must success")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_types() {
        derive_wallet(0);
        derive_wallet(1);
        derive_wallet(u32::MAX).address();
    }
}
