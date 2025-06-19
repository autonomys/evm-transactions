import { Wallet } from 'ethers';

export interface LoadTestConfig {
  duration: number; // Test duration in seconds
  accountCount: number; // Number of accounts to use
  rpcUrl: string; // RPC endpoint URL
  chainId: number; // Chain ID
  loadContractAddress: string; // Address of the Load contract
  arraySize: number; // Size of array for Load contract
  keysFile: string; // Path to the keys file
}

export interface Account {
  wallet: Wallet;
  nonce: number | undefined;
}

export interface TransactionResult {
  hash: string;
  from: string;
  success: boolean;
  error?: string;
}

export interface TestResults {
  totalTransactions: number;
  successfulTransactions: number;
  failedTransactions: number;
  transactionsPerSecond: number;
  errors: Array<{ account: string; error: string }>;
}

export interface StoredAccount {
  address: string;
  privateKey: string;
}

export interface KeyStore {
  accounts: StoredAccount[];
  createdAt: string;
  chainId: number;
}
