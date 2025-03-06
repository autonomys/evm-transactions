import { Command } from 'commander';
import { JsonRpcProvider, parseEther, Wallet } from 'ethers';
import { config as dotenvConfig } from 'dotenv';
import * as path from 'path';
import * as fs from 'fs/promises';
import { Account, KeyStore, TestResults } from './types';
import { loadAccounts } from './accountManager';
import createLogger from './logger';

dotenvConfig();

const MAX_RETRIES = 3;
const BASE_FEE = 1000000000n; // 1 gwei
const PRIORITY_FEE = 100000000n; // 0.1 gwei
const FEE_ESCALATION_FACTOR = 1.2; // 20% increase per retry
const TRANSFER_GAS_LIMIT = 21000n; // Standard gas limit for ETH transfers
const BATCH_SIZE = 1500; // Number of concurrent transactions per batch
const LOG_BATCH_SIZE = 1000; // Number of transactions to accumulate before writing to log

const program = new Command();

program
  .name('transfer-load-test')
  .description('EVM transfer load testing tool')
  .requiredOption('-d, --duration <seconds>', 'Test duration in seconds', parseInt)
  .requiredOption('-k, --keys-file <path>', 'Path to the keys file in the keys directory')
  .requiredOption('-t, --to <address>', 'Address to transfer to')
  .requiredOption('-a, --amount <ether>', 'Amount in ETH to transfer per transaction')
  .parse();

interface TransferResult {
  hash: string;
  from: string;
  success: boolean;
  error?: string;
}

const executeTransfer = async (
  account: Account,
  recipient: string,
  amount: bigint,
  retries = 0
): Promise<TransferResult> => {
  try {
    // Always get fresh nonce from network
    account.nonce = await account.wallet.getNonce();

    if (!account.wallet.provider) {
      throw new Error('Provider not connected');
    }

    // Calculate escalated fees based on retry count
    const escalationMultiplier = Math.pow(FEE_ESCALATION_FACTOR, retries);
    const maxFeePerGas = BigInt(Math.floor(Number(BASE_FEE) * escalationMultiplier));
    const maxPriorityFeePerGas = BigInt(Math.floor(Number(PRIORITY_FEE) * escalationMultiplier));

    // Check if account has enough balance
    const balance = await account.wallet.provider.getBalance(account.wallet.address);
    const estimatedGasCost = TRANSFER_GAS_LIMIT * maxFeePerGas;
    const totalRequired = estimatedGasCost + amount;

    if (balance < totalRequired) {
      return {
        hash: '',
        from: account.wallet.address,
        success: false,
        error: `Insufficient balance: ${balance} < ${totalRequired} required (gas: ${estimatedGasCost}, value: ${amount})`,
      };
    }

    const tx = await account.wallet.sendTransaction({
      to: recipient,
      value: amount,
      nonce: account.nonce,
      maxFeePerGas,
      maxPriorityFeePerGas,
      gasLimit: TRANSFER_GAS_LIMIT,
    });

    const receipt = await tx.wait();
    if (!receipt) throw new Error('No receipt received');

    return {
      hash: receipt.hash,
      from: account.wallet.address,
      success: true,
    };
  } catch (error: any) {
    if (
      retries < MAX_RETRIES &&
      (error.code === 'REPLACEMENT_UNDERPRICED' ||
        error.message.includes('replacement transaction underpriced') ||
        error.message.includes('nonce too low') ||
        error.message.includes('already known'))
    ) {
      await new Promise(resolve => setTimeout(resolve, 1000 * (retries + 1)));
      return executeTransfer(account, recipient, amount, retries + 1);
    }

    return {
      hash: error.transaction?.hash || '',
      from: account.wallet.address,
      success: false,
      error: error.info?.error?.message || error.message,
    };
  }
};

const processBatch = async (
  accounts: Account[],
  recipient: string,
  amount: bigint,
  results: TestResults,
  logger: ReturnType<typeof createLogger>,
  pendingLogs: any[]
) => {
  const batchAccounts = accounts.slice(0, BATCH_SIZE);
  const promises = batchAccounts.map(async account => {
    const txStartTime = Date.now();
    const result = await executeTransfer(account, recipient, amount);
    const txDuration = Date.now() - txStartTime;

    // Store log entry
    pendingLogs.push({
      timestamp: new Date().toISOString(),
      from: account.wallet.address,
      hash: result.hash,
      success: result.success,
      error: result.error,
      nonce: account.nonce,
      durationMs: txDuration,
    });

    if (result.success) {
      account.nonce++;
      results.successfulTransactions++;
    } else {
      results.failedTransactions++;
      results.errors.push({
        account: account.wallet.address,
        error: result.error || 'Unknown error',
      });
    }
    results.totalTransactions++;
  });

  await Promise.all(promises);

  // Write accumulated logs if threshold reached
  if (pendingLogs.length >= LOG_BATCH_SIZE) {
    pendingLogs.forEach(log => logger.info('Transfer completed', log));
    pendingLogs.length = 0; // Clear array while maintaining reference
  }

  // Move processed accounts to the end of the array
  accounts.push(...accounts.splice(0, BATCH_SIZE));
};

const runTransferLoadTest = async () => {
  const opts = program.opts();
  const testId = new Date().toISOString().replace(/[:.]/g, '-');
  const logger = createLogger(testId);

  // Validate and read keys file
  const keysPath = path.join('keys', opts.keysFile);
  console.log('Loading accounts from:', keysPath);

  const provider = new JsonRpcProvider(process.env.RPC_URL);
  const accounts = await loadAccounts(keysPath, provider);
  const transferAmount = parseEther(opts.amount);

  logger.info('Starting transfer load test', {
    duration: opts.duration,
    accountCount: accounts.length,
    recipient: opts.to,
    amountEth: opts.amount,
    rpcUrl: process.env.RPC_URL?.split('?')[0], // Remove any API keys from URL
  });

  console.log('\nStarting transfer load test with configuration:');
  console.log('--------------------------------------------');
  console.log(`Duration: ${opts.duration} seconds`);
  console.log(`Account Count: ${accounts.length}`);
  console.log(`Transfer Amount: ${opts.amount} ETH`);
  console.log(`Recipient: ${opts.to}`);
  console.log(`Using accounts from: ${keysPath}`);
  console.log(`Batch Size: ${BATCH_SIZE} concurrent transactions`);
  console.log('\nPress Ctrl+C to stop the test\n');

  const results: TestResults = {
    totalTransactions: 0,
    successfulTransactions: 0,
    failedTransactions: 0,
    transactionsPerSecond: 0,
    errors: [],
  };

  const startTime = Date.now();
  const endTime = startTime + opts.duration * 1000;
  let lastLogTime = startTime;
  const pendingLogs: any[] = [];

  try {
    while (Date.now() < endTime) {
      await processBatch(accounts, opts.to, transferAmount, results, logger, pendingLogs);

      // Update progress every second
      const now = Date.now();
      if (now - lastLogTime >= 1000) {
        const elapsedSeconds = (now - startTime) / 1000;
        const progress = ((elapsedSeconds / opts.duration) * 100).toFixed(1);
        const currentTps = (results.totalTransactions / elapsedSeconds).toFixed(2);
        process.stdout.write(
          `\rProgress: ${progress}% | Transactions: ${results.totalTransactions} | ` +
            `Success: ${results.successfulTransactions} | Failed: ${results.failedTransactions} | ` +
            `Current TPS: ${currentTps}`
        );
        lastLogTime = now;

        // Force garbage collection if available
        if (global.gc) {
          global.gc();
        }
      }
    }
  } finally {
    // Write any remaining logs
    if (pendingLogs.length > 0) {
      pendingLogs.forEach(log => logger.info('Transfer completed', log));
    }
  }

  const duration = (Date.now() - startTime) / 1000;
  results.transactionsPerSecond = results.totalTransactions / duration;

  logger.info('Transfer load test completed', {
    results,
    durationSeconds: duration,
  });

  // Clear the progress line and add a newline
  process.stdout.write('\n\n');

  console.log('Load Test Results:');
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

  console.log(`\nDetailed logs written to: logs/loadtest-${testId}.log`);
};

runTransferLoadTest().catch(error => {
  console.error('Error running transfer load test:', error);
  process.exit(1);
});
