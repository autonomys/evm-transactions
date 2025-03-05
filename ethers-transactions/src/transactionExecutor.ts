import { Contract } from 'ethers';
import { Account, TransactionResult, LoadTestConfig } from './types';
import LoadABI from './abi/Load.json';

export const executeTransaction = async (account: Account, config: LoadTestConfig): Promise<TransactionResult> => {
  const loadContract = new Contract(config.loadContractAddress, LoadABI, account.wallet);

  try {
    const tx = await loadContract.setArray(config.arraySize, {
      nonce: account.nonce,
    });
    const receipt = await tx.wait();

    return {
      hash: receipt.hash,
      from: account.wallet.address,
      success: true,
    };
  } catch (error: any) {
    return {
      hash: '',
      from: account.wallet.address,
      success: false,
      error: error.message,
    };
  }
};
