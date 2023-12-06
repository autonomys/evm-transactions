use ethers::prelude::*;
use eyre::{Report, Result};
use log::{debug, error, info};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

// Constants for retry strategy
const MAX_RETRIES: u32 = 2;
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct TransactionManager {
    pub client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
    pub wallet: LocalWallet,
    pub chain_id: u64,
    num_confirmations: usize,
}

impl TransactionManager {
    pub fn new(
        provider: Arc<Provider<Http>>,
        wallet: &LocalWallet,
        num_confirmations: usize,
    ) -> Self {
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        //TODO: This is a hack to get the chain_id from the wallet, will replace in the future
        let chain_id = wallet.chain_id();
        TransactionManager {
            client,
            wallet: wallet.clone(),
            chain_id,
            num_confirmations,
        }
    }

    pub async fn handle_transaction(&self, transaction: TransactionRequest) -> Result<(), Report> {
        let mut attempts = 0;
        let mut adjust_gas_price = false;

        while attempts < MAX_RETRIES {
            let transaction = if adjust_gas_price {
                let gas_price = transaction.gas_price.unwrap_or_default();
                let new_gas_price = gas_price.checked_add(100.into()).unwrap_or_default();
                info!(
                    "Attempt #{:?} Gas price was {:?}, will retry with gas price {:?} for wallet {:?}.",
                    attempts,
                    gas_price,
                    &new_gas_price,
                    self.get_address(),
                );
                transaction.clone().gas_price(new_gas_price)
            } else {
                transaction.clone()
            };

            match self.try_send_transaction(&transaction).await {
                Ok(()) => return Ok(()),
                Err(e) if attempts < MAX_RETRIES => {
                    if e.to_string().contains("already known") {
                        info!(
                            "Transaction {:?} already known, retrying with new nonce {:?}",
                            transaction, transaction.nonce
                        );
                        adjust_gas_price = true;
                    };

                    error!(
                        "Error sending transaction, retry #{:?} from wallet {:?}: {:?}",
                        attempts + 1,
                        self.get_address(),
                        e,
                    );

                    sleep(RETRY_DELAY * (attempts + 1)).await;
                    attempts += 1;
                }
                Err(e) => {
                    error!("Error sending transaction, giving up: {:?}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    async fn try_send_transaction(&self, transaction: &TransactionRequest) -> Result<(), Report> {
        debug!("Sending transaction {:?}", transaction);
        match self
            .client
            .send_transaction(transaction.clone(), None)
            .await
        {
            Ok(pending_tx) => {
                let tx_hash = pending_tx.tx_hash();
                info!(
                    "Transaction {:?} sent from wallet {:?}. Waiting for confirmation...",
                    tx_hash,
                    self.get_address()
                );

                let receipt = pending_tx
                    .confirmations(self.num_confirmations)
                    .await?
                    .unwrap_or_default();

                info!(
                    "Transaction {:?} confirmed. Block #{:?} ({:?})",
                    tx_hash, receipt.block_number, receipt.block_hash
                );
            }
            Err(e) => {
                error!("Error sending transaction: {:?}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub fn get_address(&self) -> Address {
        self.wallet.address()
    }
}
