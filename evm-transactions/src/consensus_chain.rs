#![allow(missing_docs)]
use eyre::Result;
use futures::future::join_all;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use subspace::balances::calls::types::TransferKeepAlive;
use subxt::backend::StreamOfResults;
use subxt::blocks::Block;
use subxt::config::substrate::{SubstrateExtrinsicParams, SubstrateHeader};
use subxt::tx::TxClient;
use subxt::tx::TxStatus;
use subxt::{OnlineClient, SubstrateConfig};
use subxt_signer::sr25519::{dev, Keypair};

// TODO: transfer by size limit: 75618 transfer
// TODO: transfer by weight limit: 6572 transfer
// TODO: pool limit by default: 8192 tx, 20MB

const TX_PER_BATCH: u32 = 5000;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "./artifacts/metadata.scale")]
pub mod subspace {}

pub async fn consensus_chain_load_test(
    consensus_url: String,
    tx_count: u32,
    concurrencies: u32,
) -> Result<()> {
    // Create a new API client, configured to talk to Subspace nodes.
    let api = OnlineClient::<SubstrateConfig>::from_url(&consensus_url).await?;
    let tx_client = api.tx();

    let mut blocks_stream = api.blocks().subscribe_best().await?;
    let latest_block = blocks_stream
        .next()
        .await
        .expect("unexpected ended stream")?;

    let from = dev::alice();

    // Pre-fund account
    pre_fund_account(&from, tx_count, &tx_client, &latest_block).await?;

    let block_monitor_fut = block_monitor(blocks_stream, tx_count);
    let transfer_fut = transfer_fund(tx_count, tx_client, Arc::new(latest_block), concurrencies);

    let (block_monitor_res, transfer_res) = tokio::join!(block_monitor_fut, transfer_fut);
    if let Err(err) = block_monitor_res {
        log::info!("Block monitor failed, err {err:?}");
    }
    if let Err(err) = transfer_res {
        log::info!("Transfer failed, err {err:?}");
    }

    Ok(())
}

async fn pre_fund_account(
    funding_account: &Keypair,
    tx_count: u32,
    tx_client: &TxClient<SubstrateConfig, OnlineClient<SubstrateConfig>>,
    latest_block: &Block<SubstrateConfig, OnlineClient<SubstrateConfig>>,
) -> Result<()> {
    let mut from_account_nonce = latest_block
        .account_nonce(&funding_account.public_key().into())
        .await?;

    let mut last_tx_progress = None;
    let mut calls = vec![];
    for i in 0..tx_count {
        let call = subspace::Call::Balances(subspace::balances::Call::transfer_keep_alive {
            dest: derive_key_pair(i as u64).public_key().into(),
            value: 2_000_000_000_000,
        });
        calls.push(call);
        if (i != 0 && i % TX_PER_BATCH == 0) || i == tx_count - 1 {
            let batch_tx = subspace::tx().utility().batch_all(mem::take(&mut calls));
            let tx_progress = tx_client
                .create_signed_with_nonce(
                    &batch_tx,
                    funding_account,
                    from_account_nonce,
                    Default::default(),
                )?
                .submit_and_watch()
                .await?;

            last_tx_progress.replace(tx_progress);
            from_account_nonce += 1;
        }
    }

    if let Some(mut tx_progress) = last_tx_progress {
        while let Some(state) = tx_progress.next().await {
            match state? {
                TxStatus::InBestBlock(block) | TxStatus::InFinalizedBlock(block) => {
                    log::info!("Last batch transfer in block {:?}", block.block_hash());
                    break;
                }
                s => log::info!("Last batch transfer state: {s:?}"),
            };
        }
    }

    Ok(())
}

async fn transfer_fund(
    tx_count: u32,
    tx_client: TxClient<SubstrateConfig, OnlineClient<SubstrateConfig>>,
    latest_block: Arc<Block<SubstrateConfig, OnlineClient<SubstrateConfig>>>,
    concurrencies: u32,
) -> Result<()> {
    let transfer_per_fut = tx_count / concurrencies;
    let mut submit_transfer_futs = vec![];
    for i in 0..concurrencies {
        let start = i * transfer_per_fut;
        let tx_client = tx_client.clone();
        let latest_block = latest_block.clone();
        let mut submitted_transfer = 0;
        let fut = async move {
            for j in 0..transfer_per_fut {
                let idx = start + j;
                let sender = derive_key_pair(idx as u64);
                let balance_transfer_tx = {
                    let amount = match std::time::UNIX_EPOCH.elapsed() {
                        Ok(d) => d.as_millis(),
                        Err(_) => continue,
                    };
                    let receiver = derive_key_pair(u64::MAX - idx as u64).public_key();
                    subspace::tx()
                        .balances()
                        .transfer_keep_alive(receiver.into(), amount as u128)
                };
                let sender_nonce = latest_block
                    .account_nonce(&sender.public_key().into())
                    .await
                    .expect("");
                if let Ok(tx) = tx_client.create_signed_with_nonce(
                    &balance_transfer_tx,
                    &sender,
                    sender_nonce,
                    Default::default(),
                ) {
                    if let Ok(_) = tx.submit().await {
                        submitted_transfer += 1;
                    }
                }

                if j % 100 == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            submitted_transfer
        };
        submit_transfer_futs.push(fut);
    }

    let mut submitted_transfer = 0;
    for tx in join_all(submit_transfer_futs).await {
        submitted_transfer += tx;
    }
    log::info!(
        "All transfer submitted, total_transfer {:?}, submitted_transfer {submitted_transfer:?}",
        transfer_per_fut * concurrencies
    );

    Ok(())
}

async fn block_monitor(
    mut blocks_stream: StreamOfResults<Block<SubstrateConfig, OnlineClient<SubstrateConfig>>>,
    total_transfer: u32,
) -> Result<()> {
    let mut total_transfer_in_block = 0;
    while let Some(block) = blocks_stream.next().await {
        let block = block?;
        let (hash, number) = (block.hash(), block.number());

        let transfer_in_block = block
            .extrinsics()
            .await?
            .find::<TransferKeepAlive>()
            .filter_map(|e| e.ok())
            .count();

        let block_created_at = block
            .runtime_api()
            .await?
            .call(subspace::apis().domains_api().timestamp())
            .await? as u128;
        let transfer_latency = block
            .events()
            .await?
            .find::<subspace::balances::events::Transfer>()
            .filter_map(|e| e.ok())
            .map(|e| block_created_at.saturating_sub(e.amount))
            .collect::<Vec<_>>();
        let total_success_event = transfer_latency.len();

        log::info!("Block {number:?}#{hash:?}:");
        log::info!(
            "Transfer tx in block {transfer_in_block}, success transfer {total_success_event}"
        );
        if transfer_in_block != 0 && total_success_event != 0 {
            let min = transfer_latency.iter().min();
            let max = transfer_latency.iter().max();
            let avg = transfer_latency.iter().sum::<u128>() / total_success_event as u128;
            log::info!("Latency: min {min:?}ms, max {max:?}ms, avg {avg:?}ms");
        }

        total_transfer_in_block += transfer_in_block as u32;
        if total_transfer_in_block >= total_transfer {
            log::info!("All transfers in block");
            break;
        }
    }
    Ok(())
}

fn derive_key_pair(i: u64) -> Keypair {
    let seed = {
        let mut s = [0u8; 32];
        (s[..8]).copy_from_slice(&i.to_be_bytes()[..]);
        s
    };
    Keypair::from_seed(seed).expect("fail to derive key pair")
}
