use ethers::prelude::*;
use eyre::{Report, Result};
use log::{error, info};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

// Constants for retry strategy
const MAX_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct TransactionManager {
    pub client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
    pub wallet: LocalWallet,
}

impl TransactionManager {
    pub fn new(provider: Arc<Provider<Http>>, wallet: &LocalWallet) -> Self {
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        TransactionManager {
            client,
            wallet: wallet.clone(),
        }
    }

    pub async fn handle_transaction(&self, transaction: TransactionRequest) -> Result<(), Report> {
        let mut attempts = 0;
        let mut adjust_nonce = false;

        while attempts < MAX_RETRIES {
            let transaction = if adjust_nonce {
                let num_transactions = self
                    .client
                    .get_transaction_count(self.get_address(), None)
                    .await?;
                let new_nonce = num_transactions + attempts - 2; // testing if nonce got skipped due to reorg
                info!(
                    "Attempt #{:?} Will retry with nonce {:?} for wallet {:?}. Chain nonce: {:?}",
                    attempts,
                    &new_nonce,
                    self.get_address(),
                    num_transactions
                );
                transaction.clone().nonce(new_nonce)
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
                        adjust_nonce = true;
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
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    async fn try_send_transaction(&self, transaction: &TransactionRequest) -> Result<(), Report> {
        match self
            .client
            .send_transaction(transaction.clone(), None)
            .await
        {
            Ok(pending_tx) => {
                let tx_hash = pending_tx.tx_hash();
                info!(
                    "Transaction {:?} sent with {:?} nonce from wallet {:?}. Waiting for confirmation...",
                    tx_hash, transaction.nonce, self.get_address()
                );

                let _receipt = pending_tx.confirmations(1).await;
                info!("Transaction {:?} confirmed", tx_hash);
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
