import { Wallet, JsonRpcProvider, Contract } from 'ethers';
import * as fs from 'fs/promises';
import { Account, LoadTestConfig, KeyStore } from './types';
import FundABI from './abi/Fund.json';
import { parseEther } from 'ethers';

export const generateAccounts = async (
  count: number,
  provider: JsonRpcProvider,
  keysFile?: string
): Promise<Account[]> => {
  if (keysFile) {
    try {
      const fileContent = await fs.readFile(keysFile, 'utf-8');
      const keyStore: KeyStore = JSON.parse(fileContent);
      
      // Validate chain ID
      const networkId = await provider.getNetwork();
      if (networkId.chainId !== BigInt(keyStore.chainId)) {
        throw new Error(`Chain ID mismatch. Expected ${keyStore.chainId}, got ${networkId.chainId}`);
      }

      return keyStore.accounts.slice(0, count).map(account => ({
        wallet: new Wallet(account.privateKey, provider),
        nonce: 0
      }));
    } catch (error) {
      console.error('Error loading accounts from file:', error);
      throw error;
    }
  }

  // Fall back to generating random accounts if no keys file provided
  return Array.from({ length: count }, () => ({
    wallet: Wallet.createRandom().connect(provider) as unknown as Wallet,
    nonce: 0
  }));
};

export const fundAccounts = async (
  accounts: Account[],
  config: LoadTestConfig,
  funderWallet: Wallet
): Promise<void> => {
  // Skip funding if using pre-funded accounts from keys file
  if (config.keysFile) {
    console.log('Using pre-funded accounts from keys file');
    return;
  }

  const fundContract = new Contract(config.fundContractAddress, FundABI, funderWallet);

  // Fund accounts in batches of 150 as per contract limitation
  for (let i = 0; i < accounts.length; i += 150) {
    const batch = accounts.slice(i, Math.min(i + 150, accounts.length));
    const addresses = batch.map(account => account.wallet.address);

    try {
      const tx = await fundContract.transferTsscToMany(addresses, {
        value: parseEther("0.1") * BigInt(addresses.length)
      });
      await tx.wait();
      console.log(`Funded batch of ${addresses.length} accounts`);
    } catch (error) {
      console.error(`Error funding accounts batch ${i}-${i + batch.length}:`, error);
      throw error;
    }
  }
};
