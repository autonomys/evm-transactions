import { Wallet, JsonRpcProvider } from 'ethers';
import * as fs from 'fs/promises';
import { Account, KeyStore } from './types';

export const loadAccounts = async (
  keysFile: string,
  provider: JsonRpcProvider
): Promise<Account[]> => {
  console.log('Loading accounts from:', keysFile);
  
  try {
    const fileContent = await fs.readFile(keysFile, 'utf-8');
    const keyStore: KeyStore = JSON.parse(fileContent);
    
    // Validate chain ID
    const networkId = await provider.getNetwork();
    if (networkId.chainId !== BigInt(keyStore.chainId)) {
      throw new Error(`Chain ID mismatch. Expected ${keyStore.chainId}, got ${networkId.chainId}`);
    }

    const accounts = keyStore.accounts.map(account => ({
      wallet: new Wallet(account.privateKey, provider),
      nonce: 0
    }));

    console.log(`Successfully loaded ${accounts.length} accounts`);
    return accounts;
  } catch (error) {
    console.error('Error loading accounts:', error);
    throw error;
  }
};
