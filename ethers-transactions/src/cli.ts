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
  .option('-s, --array-size <size>', 'Size of array for Load contract', parseInt)
  .requiredOption('-k, --keys-file <path>', 'Path to the keys file in the keys directory')
  .parse();

const main = async () => {
  try {
    const opts = program.opts();

    // Validate and read keys file
    const keysPath = path.join('keys', opts.keysFile);
    console.log('Loading accounts from:', keysPath);

    let accountCount: number;
    try {
      const keyStore: KeyStore = JSON.parse(await fs.readFile(keysPath, 'utf-8'));
      accountCount = keyStore.accounts.length;
      console.log(`Found ${accountCount} accounts in keys file`);
    } catch (error) {
      console.error('Error reading keys file:', error);
      process.exit(1);
    }

    const cliArgs: CLIArgs = {
      duration: opts.duration,
      accountCount,
      arraySize: opts.arraySize,
      keysFile: keysPath,
    };

    const config = loadConfig(cliArgs);

    console.log('Starting load test with configuration:');
    console.log('------------------------------------');
    console.log(`Duration: ${config.duration} seconds`);
    console.log(`Account Count: ${config.accountCount}`);
    console.log(`Array Size: ${config.arraySize}`);
    console.log(`Chain ID: ${config.chainId}`);
    console.log(`RPC URL: ${config.rpcUrl}`);
    console.log(`Load Contract: ${config.loadContractAddress}`);
    console.log(`Using accounts from: ${config.keysFile}`);
    console.log('------------------------------------\n');

    const results = await runLoadTest(config);

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
