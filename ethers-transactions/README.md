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

The generate-keys script creates new accounts and automatically funds them using the Fund contract:

```bash
# Generate 20 accounts and save to keys/my-accounts.json
yarn generate-keys -n 20 -o my-accounts.json

# Generate 50 accounts with 0.1 tokens each
yarn generate-keys -n 50 -f 0.1 -o my-accounts.json
```

**Important**: The funder wallet must have sufficient balance to cover:

- Total funding amount: `fund-amount × num-accounts`
- Gas fees for the funding transactions
- Accounts are funded in batches of up to 150 addresses per transaction

Options:

- `-n, --num-accounts`: Number of accounts to generate (default: 10)
- `-o, --output`: Output filename in the keys directory (default: accounts.json)
- `-f, --fund-amount`: Amount of tokens to fund each account (default: 1)

**Troubleshooting**:

- If you get "OutOfFund" errors, ensure your funder wallet has enough balance
- Reduce the fund amount per account (`-f 0.1`) or number of accounts (`-n 10`) if needed
- The script will create unique filenames if the output file already exists

### Run Load Tests

There are two types of load tests available:

#### Contract Load Test

Tests the Load contract's `setArray` function with multiple accounts:

```bash
# Run contract load test for 5 minutes with array size 200
yarn load -d 300 -s 200 -k my-accounts.json
```

Options:

- `-d, --duration <seconds>`: Test duration in seconds (required)
- `-s, --array-size <size>`: Size of array for Load contract (required)
- `-k, --keys-file <path>`: Path to keys file in keys directory (required)

The test automatically uses all accounts from the specified keys file.

#### Transfer Load Test

Tests native token transfers between accounts:

```bash
# Run transfer load test - send 0.01 tokens to specific address for 5 minutes
yarn transfer-load-test -d 300 -k my-accounts.json -t 0x518EbE66287140e9378b9F8D00797291A8dfc2bc -a 0.01
```

Options:

- `-d, --duration <seconds>`: Test duration in seconds (required)
- `-k, --keys-file <path>`: Path to keys file in keys directory (required)
- `-t, --to <address>`: Recipient address for transfers (required)
- `-a, --amount <tokens>`: Amount of tokens to transfer per transaction (required)

**Performance Features**:

- Processes transactions in batches of 1,500 concurrent operations
- Automatic fee escalation on retry (20% increase per attempt)
- Up to 3 retry attempts for failed transactions
- Real-time logging and progress tracking

**Important Notes**:

- **Contract Load Test**: Processes accounts in batches of 10 to prevent RPC overload
- **Legacy Transactions**: Both tests use legacy transaction format (gasPrice) for chains that don't support EIP-1559
- **Concurrency**: Reduce batch sizes if you experience RPC timeouts or hanging transactions
- **Gas Estimation**: Transactions automatically retry with higher gas prices on failure

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
