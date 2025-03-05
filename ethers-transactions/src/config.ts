import { config as dotenvConfig } from 'dotenv';
import { LoadTestConfig } from './types';

dotenvConfig();

const getEnvVar = (key: string, defaultValue?: string): string => {
  const value = process.env[key];
  if (!value && defaultValue === undefined) {
    throw new Error(`Environment variable ${key} is required but not set`);
  }
  return value || defaultValue as string;
};

export interface CLIArgs {
  duration?: number;
  accountCount?: number;
  arraySize?: number;
  keysFile?: string;
}

export const loadConfig = (cliArgs: CLIArgs): LoadTestConfig => ({
  duration: cliArgs.duration ?? parseInt(getEnvVar('TEST_DURATION', '60')),
  accountCount: cliArgs.accountCount ?? parseInt(getEnvVar('ACCOUNT_COUNT', '10')),
  rpcUrl: getEnvVar('RPC_URL'),
  chainId: parseInt(getEnvVar('CHAIN_ID')),
  fundContractAddress: getEnvVar('FUND_CONTRACT_ADDRESS'),
  loadContractAddress: getEnvVar('LOAD_CONTRACT_ADDRESS'),
  arraySize: cliArgs.arraySize ?? parseInt(getEnvVar('ARRAY_SIZE', '100')),
});
