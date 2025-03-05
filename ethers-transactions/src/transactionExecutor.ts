import { Contract } from 'ethers';
import { Account, TransactionResult, LoadTestConfig } from './types';
import LoadABI from './abi/Load.json';

const MAX_RETRIES = 3;
const GAS_BUFFER_PERCENTAGE = 20n; // Add 20% to estimated gas

export const executeTransaction = async (account: Account, config: LoadTestConfig): Promise<TransactionResult> => {
  const loadContract = new Contract(config.loadContractAddress, LoadABI, account.wallet);
  let retries = 0;

  while (retries < MAX_RETRIES) {
    try {
      // Always get fresh nonce from network
      account.nonce = await account.wallet.getNonce();

      if (!account.wallet.provider) {
        throw new Error('Provider not connected');
      }

      // Estimate gas for this specific transaction
      let gasLimit;
      try {
        gasLimit = await loadContract.setArray.estimateGas(config.arraySize);
        // Add buffer to estimated gas
        gasLimit = (gasLimit * (100n + GAS_BUFFER_PERCENTAGE)) / 100n;
      } catch (error: any) {
        return {
          hash: '',
          from: account.wallet.address,
          success: false,
          error: `Gas estimation failed: ${error.message}`,
        };
      }

      // Check if account has enough balance for estimated gas
      const balance = await account.wallet.provider.getBalance(account.wallet.address);
      const estimatedGasCost = gasLimit * 1000000000n; // Using 1 gwei max fee

      if (balance < estimatedGasCost) {
        return {
          hash: '',
          from: account.wallet.address,
          success: false,
          error: `Insufficient balance: ${balance} < ${estimatedGasCost} required`,
        };
      }

      const tx = await loadContract.setArray(config.arraySize, {
        nonce: account.nonce,
        maxFeePerGas: 1000000000n, // 1 gwei
        maxPriorityFeePerGas: 100000000n, // 0.1 gwei
        gasLimit,
      });

      const receipt = await tx.wait();
      return {
        hash: receipt.hash,
        from: account.wallet.address,
        success: true,
      };
    } catch (error: any) {
      retries++;

      // If it's the last retry, return the error
      if (retries === MAX_RETRIES) {
        const errorMessage = error.info?.error?.message || error.message;
        return {
          hash: error.transaction?.hash || '',
          from: account.wallet.address,
          success: false,
          error: errorMessage,
        };
      }

      // If it's a nonce or already known error, wait a bit before retrying
      if (error.message.includes('nonce') || error.message.includes('already known')) {
        await new Promise(resolve => setTimeout(resolve, 1000 * retries));
        continue;
      }

      // For other errors, throw immediately
      throw error;
    }
  }

  // This should never happen due to the while loop condition
  throw new Error('Unexpected end of transaction execution');
};
