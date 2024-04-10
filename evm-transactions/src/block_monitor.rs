#![allow(missing_docs)]
use codec::{Decode, Encode};
use evm_domain::ethereum::calls::types::Transact;
use std::mem;
use std::time::Duration;
use subspace::balances::calls::types::TransferKeepAlive;
use subspace::domains::calls::types::SubmitBundle;
use subspace::runtime_types::sp_domains::DomainId;
use subxt::backend::BlockRef;
use subxt::backend::StreamOfResults;
use subxt::blocks::Block;
use subxt::blocks::BlocksClient;
use subxt::config::substrate::BlakeTwo256;
use subxt::config::substrate::{Digest, DigestItem};
use subxt::config::substrate::{SubstrateExtrinsicParams, SubstrateHeader};
use subxt::tx::TxClient;
use subxt::tx::TxStatus;
use subxt::Config;
use subxt::{OnlineClient, SubstrateConfig};

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "./artifacts/metadata.scale")]
pub mod subspace {}

#[subxt::subxt(runtime_metadata_path = "./artifacts/evm-domain-metadata.scale")]
pub mod evm_domain {}

pub async fn domain_block_monitor(
    domain_id: u32,
    consensus_url: String,
    domain_url: String,
    total_transfer: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let consensus_api = OnlineClient::<SubstrateConfig>::from_url(&consensus_url).await?;
    let consensus_client = consensus_api.blocks();
    let domain_api = OnlineClient::<SubstrateConfig>::from_url(&domain_url).await?;
    let domain_client = domain_api.blocks();

    let mut total_transfer_in_block = 0;
    let mut blocks_stream = domain_client.subscribe_best().await?;
    while let Some(block) = blocks_stream.next().await {
        let domain_block = block?;
        let consensus_block = {
            let raw_consensus_hash: [u8; 32] = domain_block
                .header()
                .digest
                .logs
                .iter()
                .find_map(|digest_item| {
                    if let DigestItem::PreRuntime(id, data) = digest_item {
                        if &id == &b"RGTR" {
                            return Some(data.clone());
                        }
                    }
                    None
                })
                .expect("Consensus hash digest must exist")
                .try_into()
                .expect("Must success");
            consensus_client
                .at(BlockRef::from_hash(raw_consensus_hash.into()))
                .await?
        };
        let (domain_hash, domain_number) = (domain_block.hash(), domain_block.number());
        let (consensus_hash, consensus_number) = (consensus_block.hash(), consensus_block.number());

        let transfer_in_block = domain_block
            .extrinsics()
            .await?
            .find::<Transact>()
            .filter_map(|e| e.ok())
            .collect::<Vec<_>>();
        let transfer_tx_count = transfer_in_block.len();
        let transfer_tx_size = transfer_in_block
            .iter()
            .map(|tx| tx.value.encoded_size())
            .sum::<usize>();

        let success_transfer = domain_block
            .events()
            .await?
            .find::<evm_domain::balances::events::Transfer>()
            .filter_map(|e| e.ok())
            .count();

        let bundle_in_block = consensus_block
            .extrinsics()
            .await?
            .find::<SubmitBundle>()
            .filter_map(|e| e.ok())
            .map(|submit_bundle| submit_bundle.value.opaque_bundle)
            .collect::<Vec<_>>();
        let total_bundle_count = bundle_in_block.len();

        let domain_bundle = bundle_in_block
            .iter()
            .filter(|bundle| bundle.sealed_header.header.proof_of_election.domain_id.0 == domain_id)
            .collect::<Vec<_>>();
        let domain_bundle_count = domain_bundle.len();
        let domain_bundle_tx_count = domain_bundle
            .iter()
            .map(|bundle| bundle.extrinsics.len())
            .sum::<usize>();
        let domain_bundle_size = domain_bundle
            .iter()
            .map(|bundle| bundle.encoded_size())
            .sum::<usize>();
        let domain_bundle_body_size = domain_bundle
            .iter()
            .map(|bundle| bundle.extrinsics.encoded_size())
            .sum::<usize>();

        if domain_bundle_count == 0 && transfer_tx_count == 0 {
            continue;
        }

        println!("Domain block {domain_number:?}#{domain_hash:?} derive from consensus block {consensus_number:?}#{consensus_hash:?}:");
        println!(
            "Total bundle in block: {total_bundle_count}, bundle of domain {domain_id}: {domain_bundle_count}"
        );
        println!(
            "Total tx in bundle: {domain_bundle_tx_count}, total transfer tx in domain block: {transfer_tx_count}, success transfer: {success_transfer}"
        );
        println!(
            "Total domain bundle size: {domain_bundle_size}, total bundle body size: {domain_bundle_body_size}, total domain block tx size: {transfer_tx_size}\n"
        );

        total_transfer_in_block += transfer_tx_count as u64;
        if total_transfer_in_block >= total_transfer {
            println!("All transfers in block");
            break;
        }
    }
    Ok(())
}
