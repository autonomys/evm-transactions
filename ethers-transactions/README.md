# EVM Load Testing Tool

A TypeScript-based tool for load testing EVM-compatible blockchains using pre-deployed test contracts.

## Prerequisites

- Node.js >= 16
- Yarn
- Access to an EVM-compatible blockchain
- Deployed Fund and Load contracts

## Setup

1. Install dependencies:

```bash
yarn install
```

2. Configure environment variables:

```bash
cp .env.example .env
```

Edit `.env` with your values:

- `RPC_URL`: Your blockchain node's RPC endpoint
- `CHAIN_ID`: Chain ID of the network
- `FUNDER_PRIVATE_KEY`: Private key with funds to distribute
- `FUND_CONTRACT_ADDRESS`: Address of the deployed Fund contract
- `LOAD_CONTRACT_ADDRESS`: Address of the deployed Load contract

## Usage

### Generate Test Accounts

First, generate and fund test accounts:

```bash
# Generate 20 accounts and save to keys/my-accounts.json
yarn generate-keys -n 20 -o my-accounts.json
```

Options:

- `-n, --num-accounts`: Number of accounts to generate (default: 10)
- `-o, --output`: Output filename in the keys directory (default: accounts.json)

### Run Load Test

Run the load test using either newly generated accounts or pre-funded accounts:

```bash
# Using pre-funded accounts
yarn load -d 300 -a 20 -s 200 -k my-accounts.json

# Using new random accounts
yarn load -d 300 -a 20 -s 200
```

Options:

- `-d, --duration`: Test duration in seconds
- `-a, --account-count`: Number of accounts to use
- `-s, --array-size`: Size of array for Load contract
- `-k, --keys-file`: Path to the generated keys file (relative to keys directory)

## Architecture

- Written in TypeScript with functional programming style
- Uses ethers.js for blockchain interaction
- Supports concurrent transactions from multiple accounts
- Manages nonces automatically
- Provides detailed test results and error reporting

## Contract Requirements

### Fund Contract

- Must have a `fundAccounts(address[])` function
- Can fund up to 150 accounts per transaction

### Load Contract

- Must have a `setArray(uint256)` function
- Used to generate variable-sized transactions

## Security

- Private keys are stored in the `keys` directory (git-ignored)
- Environment variables for sensitive configuration
- Never commit `.env` or key files to version control
