import { Command } from 'commander';
import { loadConfig, CLIArgs } from './config';
import runLoadTest from './index';
import * as path from 'path';
import * as fs from 'fs/promises';
import { KeyStore } from './types';

const program = new Command();

program
  .name('evm-load-tester')
  .description('EVM blockchain load testing tool')
  .version('1.0.0')
  .option('-d, --duration <seconds>', 'Test duration in seconds', parseInt)
  .option('-a, --account-count <number>', 'Number of accounts to use (optional if using keys file)', parseInt)
  .option('-s, --array-size <size>', 'Size of array for Load contract', parseInt)
  .option('-k, --keys-file <path>', 'Path to the keys file (relative to keys directory)')
  .parse();

const main = async () => {
  try {
    const opts = program.opts();

    // If keys file is provided, get account count from it
    let accountCount = opts.accountCount;
    if (opts.keysFile && !accountCount) {
      const keysPath = path.join('keys', opts.keysFile);
      const keyStore: KeyStore = JSON.parse(await fs.readFile(keysPath, 'utf-8'));
      accountCount = keyStore.accounts.length;
    }

    const cliArgs: CLIArgs = {
      duration: opts.duration,
      accountCount,
      arraySize: opts.arraySize,
      keysFile: opts.keysFile ? path.join('keys', opts.keysFile) : undefined,
    };

    const config = loadConfig(cliArgs);
    const funderPrivateKey = process.env.FUNDER_PRIVATE_KEY;

    if (!funderPrivateKey) {
      throw new Error('FUNDER_PRIVATE_KEY environment variable is required');
    }

    console.log('Starting load test with configuration:');
    console.log('------------------------------------');
    console.log(`Duration: ${config.duration} seconds`);
    console.log(`Account Count: ${config.accountCount}`);
    console.log(`Array Size: ${config.arraySize}`);
    console.log(`Chain ID: ${config.chainId}`);
    console.log(`RPC URL: ${config.rpcUrl}`);
    console.log(`Fund Contract: ${config.fundContractAddress}`);
    console.log(`Load Contract: ${config.loadContractAddress}`);
    if (config.keysFile) {
      console.log(`Using accounts from: ${config.keysFile}`);
    }
    console.log('------------------------------------\n');

    const results = await runLoadTest(config, funderPrivateKey);

    console.log('\nLoad Test Results:');
    console.log('------------------');
    console.log(`Total Transactions: ${results.totalTransactions}`);
    console.log(`Successful Transactions: ${results.successfulTransactions}`);
    console.log(`Failed Transactions: ${results.failedTransactions}`);
    console.log(`Transactions per Second: ${results.transactionsPerSecond.toFixed(2)}`);

    if (results.errors.length > 0) {
      console.log('\nErrors:');
      results.errors.forEach(({ account, error }) => {
        console.log(`${account}: ${error}`);
      });
    }
  } catch (error) {
    console.error('Error running load test:', error);
    process.exit(1);
  }
};

main();
