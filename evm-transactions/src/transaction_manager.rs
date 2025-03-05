use ethers::prelude::*;
use eyre::{Report, Result};
use log::{debug, error, info};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

// Constants for retry strategy
const MAX_RETRIES: u32 = 2;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const GAS_PRICE_BUMP_PERCENT: u64 = 10;
const MIN_PRIORITY_FEE: u64 = 2_000_000_000; // 2 gwei minimum tip

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

    async fn get_base_fee(&self) -> Result<U256> {
        let block = self.client.get_block(BlockNumber::Latest).await?;
        Ok(block
            .and_then(|b| b.base_fee_per_gas)
            .unwrap_or_else(|| U256::from(1_000_000_000))) // fallback to 1 gwei if no base fee
    }

    async fn ensure_sufficient_gas_price(
        &self,
        transaction: &mut TransactionRequest,
    ) -> Result<()> {
        let base_fee = self.get_base_fee().await?;
        let min_gas_price = base_fee + U256::from(MIN_PRIORITY_FEE);

        let current_gas_price = transaction.gas_price.unwrap_or_else(|| min_gas_price);

        if current_gas_price < min_gas_price {
            info!(
                "Adjusting gas price from {:?} to minimum required {:?} (base fee: {:?} + priority fee: {:?})",
                current_gas_price,
                min_gas_price,
                base_fee,
                MIN_PRIORITY_FEE
            );
            transaction.gas_price = Some(min_gas_price);
        }

        Ok(())
    }

    pub async fn handle_transaction(
        &self,
        mut transaction: TransactionRequest,
    ) -> Result<(), Report> {
        let mut attempts = 0;

        while attempts < MAX_RETRIES {
            // Get the current nonce if not set
            if transaction.nonce.is_none() {
                let nonce = self
                    .client
                    .get_transaction_count(self.get_address(), None)
                    .await?;
                transaction = transaction.clone().nonce(nonce);
            }

            // Ensure gas price is sufficient
            self.ensure_sufficient_gas_price(&mut transaction).await?;

            // On retry, increase gas price by percentage
            if attempts > 0 {
                if let Some(current_gas_price) = transaction.gas_price {
                    let bump =
                        current_gas_price * U256::from(GAS_PRICE_BUMP_PERCENT) / U256::from(100);
                    let new_gas_price = current_gas_price + bump;
                    info!(
                        "Attempt #{:?}: Bumping gas price from {:?} to {:?} for wallet {:?}",
                        attempts,
                        current_gas_price,
                        new_gas_price,
                        self.get_address()
                    );
                    transaction = transaction.clone().gas_price(new_gas_price);
                }
            }

            match self.try_send_transaction(&transaction).await {
                Ok(()) => return Ok(()),
                Err(e) if attempts < MAX_RETRIES => {
                    if e.to_string().contains("already known") {
                        // For already known transactions, increment the nonce and try again
                        if let Some(current_nonce) = transaction.nonce {
                            let new_nonce = current_nonce + U256::from(1);
                            info!(
                                "Transaction already known, incrementing nonce from {:?} to {:?}",
                                current_nonce, new_nonce
                            );
                            transaction = transaction.clone().nonce(new_nonce);
                        }
                    }

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
