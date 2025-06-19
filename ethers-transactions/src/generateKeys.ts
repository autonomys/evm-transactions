import { Command } from 'commander';
import { JsonRpcProvider, Contract, parseEther, formatEther, Wallet } from 'ethers';
import { config as dotenvConfig } from 'dotenv';
import * as fs from 'fs/promises';
import * as path from 'path';
import { KeyStore } from './types';
import FundABI from './abi/Fund.json';

dotenvConfig();

const program = new Command();

program
  .name('generate-keys')
  .description('Generate and fund accounts for load testing')
  .option('-n, --num-accounts <number>', 'Number of accounts to generate', parseInt, 10)
  .option('-o, --output <filename>', 'Output file name', 'accounts.json')
  .option('-f, --fund-amount <ether>', 'Amount of TSSC to fund each account with', '1')
  .parse();

const getUniqueFilePath = async (basePath: string): Promise<string> => {
  const dir = path.dirname(basePath);
  const ext = path.extname(basePath);
  const basename = path.basename(basePath, ext);
  let counter = 0;
  let filePath = basePath;

  while (true) {
    try {
      await fs.access(filePath);
      counter++;
      filePath = path.join(dir, `${basename}-${counter}${ext}`);
    } catch {
      return filePath;
    }
  }
};

const generateAndSaveKeys = async (count: number, chainId: number, outputPath: string) => {
  const accounts = Array.from({ length: count }, () => {
    const wallet = Wallet.createRandom();
    return {
      address: wallet.address,
      privateKey: wallet.privateKey,
    };
  });

  const keyStore: KeyStore = {
    accounts,
    createdAt: new Date().toISOString(),
    chainId,
  };

  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, JSON.stringify(keyStore, null, 2));
  return accounts;
};

const main = async () => {
  try {
    const opts = program.opts();
    const baseOutputPath = path.join('keys', opts.output);
    const outputPath = await getUniqueFilePath(baseOutputPath);
    const fundAmount = parseEther(opts.fundAmount);

    const rpcUrl = process.env.RPC_URL;
    const chainId = parseInt(process.env.CHAIN_ID || '0');
    const funderKey = process.env.FUNDER_PRIVATE_KEY;
    const fundContractAddress = process.env.FUND_CONTRACT_ADDRESS;

    if (!rpcUrl || !chainId || !funderKey || !fundContractAddress) {
      throw new Error('Missing required environment variables');
    }

    console.log(`Generating ${opts.numAccounts} accounts...`);
    const accounts = await generateAndSaveKeys(opts.numAccounts, chainId, outputPath);
    console.log(`Saved accounts to ${outputPath}`);

    const provider = new JsonRpcProvider(rpcUrl);
    const funderWallet = new Wallet(funderKey, provider);
    const fundContract = new Contract(fundContractAddress, FundABI, funderWallet);

    for (let i = 0; i < accounts.length; i += 150) {
      const batch = accounts.slice(i, Math.min(i + 150, accounts.length));
      const addresses = batch.map((account) => account.address);
      const tx = await fundContract.transferTsscToMany(addresses, {
        value: fundAmount * BigInt(addresses.length),
      });
      await tx.wait();
      console.log(
        `Funded batch of ${addresses.length} accounts with ${formatEther(fundAmount)} TSSC each`,
      );
    }

    console.log('All accounts generated and funded successfully!');
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
};

main();
