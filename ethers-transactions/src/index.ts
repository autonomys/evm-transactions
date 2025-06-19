import { JsonRpcProvider } from 'ethers';
import { LoadTestConfig, TestResults } from './types';
import { loadAccounts } from './accountManager';
import { executeTransaction } from './transactionExecutor';
import createLogger from './logger';
import * as fs from 'fs/promises';

const logProgress = (results: TestResults, elapsedSeconds: number, totalSeconds: number) => {
  const progress = ((elapsedSeconds / totalSeconds) * 100).toFixed(1);
  const currentTps = (results.totalTransactions / elapsedSeconds).toFixed(2);
  process.stdout.write(
    `\rProgress: ${progress}% | Transactions: ${results.totalTransactions} | ` +
      `Success: ${results.successfulTransactions} | Failed: ${results.failedTransactions} | ` +
      `Current TPS: ${currentTps}`,
  );
};

const runLoadTest = async (config: LoadTestConfig): Promise<TestResults> => {
  // Create logs directory if it doesn't exist
  await fs.mkdir('logs', { recursive: true });

  const testId = new Date().toISOString().replace(/[:.]/g, '-');
  const logger = createLogger(testId);

  logger.info('Starting load test', {
    config: {
      ...config,
      rpcUrl: config.rpcUrl.split('?')[0], // Remove any API keys from URL
    },
  });

  const provider = new JsonRpcProvider(config.rpcUrl);

  // Load accounts from keys file
  const accounts = await loadAccounts(config.keysFile, provider);
  logger.info('Accounts loaded', {
    count: accounts.length,
    addresses: accounts.map((a) => a.wallet.address),
  });

  console.log(`\nStarting load test with ${accounts.length} accounts...`);
  console.log('Press Ctrl+C to stop the test\n');

  const results: TestResults = {
    totalTransactions: 0,
    successfulTransactions: 0,
    failedTransactions: 0,
    transactionsPerSecond: 0,
    errors: [],
  };

  const startTime = Date.now();
  const endTime = startTime + config.duration * 1000;
  let lastLogTime = startTime;

  while (Date.now() < endTime) {
    const promises = accounts.map(async (account) => {
      const txStartTime = Date.now();
      const result = await executeTransaction(account, config);
      const txDuration = Date.now() - txStartTime;

      logger.info('Transaction completed', {
        from: account.wallet.address,
        hash: result.hash,
        success: result.success,
        error: result.error,
        nonce: account.nonce,
        durationMs: txDuration,
      });

      if (result.success) {
        account.nonce = (account.nonce || 0) + 1;
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

    // Update progress every second
    const now = Date.now();
    if (now - lastLogTime >= 1000) {
      const elapsedSeconds = (now - startTime) / 1000;
      logProgress(results, elapsedSeconds, config.duration);
      lastLogTime = now;
    }
  }

  const duration = (Date.now() - startTime) / 1000;
  results.transactionsPerSecond = results.totalTransactions / duration;

  logger.info('Load test completed', {
    results,
    durationSeconds: duration,
  });

  // Clear the progress line and add a newline
  process.stdout.write('\n\n');

  console.log(`Detailed logs written to: logs/loadtest-${testId}.log`);

  return results;
};

export default runLoadTest;
