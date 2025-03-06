import { Contract } from 'ethers';
import { Account, TransactionResult, LoadTestConfig } from './types';
import LoadABI from './abi/Load.json';

const MAX_RETRIES = 3;
const GAS_BUFFER_PERCENTAGE = 20n; // Add 20% to estimated gas
const BASE_FEE = 1000000000n; // 1 gwei
const PRIORITY_FEE = 100000000n; // 0.1 gwei
const FEE_ESCALATION_FACTOR = 1.2; // 20% increase per retry

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

      // Calculate escalated fees based on retry count
      const escalationMultiplier = Math.pow(FEE_ESCALATION_FACTOR, retries);
      const maxFeePerGas = BigInt(Math.floor(Number(BASE_FEE) * escalationMultiplier));
      const maxPriorityFeePerGas = BigInt(Math.floor(Number(PRIORITY_FEE) * escalationMultiplier));

      // Check if account has enough balance for estimated gas
      const balance = await account.wallet.provider.getBalance(account.wallet.address);
      const estimatedGasCost = gasLimit * maxFeePerGas;

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
        maxFeePerGas,
        maxPriorityFeePerGas,
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

      // Handle specific error cases
      if (
        error.code === 'REPLACEMENT_UNDERPRICED' ||
        error.message.includes('replacement transaction underpriced') ||
        error.message.includes('nonce too low') ||
        error.message.includes('already known')
      ) {
        // Wait longer for each retry
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
