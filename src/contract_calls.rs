use ethers::abi::{Abi, Tokenizable};
use ethers::prelude::*;
use ethers::types::NameOrAddress;
use ethers::utils::format_units;
use eyre::Result;
use log::info;
use serde_json::from_str;

pub(crate) fn bulk_transfer_transaction(
    to_addresses: Vec<Address>,
    funding_amount: U256,
    fund_contract_address: Address,
) -> Result<TransactionRequest> {
    let abi = include_str!("./abi/Fund.json");
    let contract_abi: Abi = from_str(abi)?;
    let function = contract_abi.function("transferTsscToMany")?;
    let args = to_addresses.clone().into_token();
    let calldata = function.encode_input(&[args])?;
    let value = funding_amount.checked_mul(U256::from(to_addresses.clone().len()));

    info!(
        "funding {:?} wallets with amount: {:?}. Total cost: {:?}",
        to_addresses.len(),
        format_units(funding_amount, 18),
        format_units(value.unwrap_or_default(), 18)
    );

    let tx_req = TransactionRequest {
        to: Some(NameOrAddress::Address(fund_contract_address)),
        value,
        data: Some(calldata.into()),
        ..Default::default()
    };

    Ok(tx_req)
}

pub(crate) fn set_array_transaction(
    load_contract_address: Address,
    count: U256,
) -> Result<TransactionRequest> {
    let abi = include_str!("./abi/Load.json");
    let contract_abi: Abi = from_str(abi)?;
    let function = contract_abi.function("setArray")?;
    let args = count.into_token();
    let calldata = function.encode_input(&[args])?;

    let tx_req = TransactionRequest {
        to: Some(NameOrAddress::Address(load_contract_address)),
        data: Some(calldata.into()),
        ..Default::default()
    };

    Ok(tx_req)
}
