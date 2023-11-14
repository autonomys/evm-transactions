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
    client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
    pub wallet: LocalWallet,
}

impl TransactionManager {
    // Create a new TransactionManager
    pub fn new(provider: Arc<Provider<Http>>, wallet: &LocalWallet) -> Self {
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        TransactionManager {
            client,
            wallet: wallet.clone(),
        }
    }

    pub async fn handle_transaction(&self, transaction: TransactionRequest) -> Result<(), Report> {
        let mut attempts = 0;

        while attempts < MAX_RETRIES {
            match self.try_send_transaction(&transaction).await {
                Ok(()) => return Ok(()),
                Err(e) if attempts < MAX_RETRIES - 1 => {
                    error!("Error sending transaction, retrying...: {:?}", e);
                    sleep(RETRY_DELAY).await;
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
                info!(
                    "Transaction {:?} sent with {:?} nonce. Waiting for confirmation...",
                    *pending_tx, transaction.nonce
                );

                let _receipt = pending_tx.confirmations(1).await;
                info!("Transaction confirmed");
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
