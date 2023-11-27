-# EVM Transaction Generator

## Description

This Rust application is designed to generate and send a specified number of transactions across the Ethereum Virtual Machine (EVM). It supports multiple accounts to distribute transactions and is configurable via environment variables and command-line arguments.

## Features

- Generate a specified number of EVM transactions.
- Support for multiple accounts to distribute the transaction load.
- Environment variable and command-line configuration.
- Integration with contract calls for funding and transaction load testing.
- Transaction management with automatic retries and error handling.

## Installation

Before you begin, ensure that Rust is installed on your system. You can install Rust through [rustup](https://rustup.rs/).

1. Clone the repository:

   ```bash
   git clone https://github.com/jfrank-summit/evm-transactions.git
   cd evm-transactions
   ```

2. Build the project:

   ```bash
   cargo build --release
   ```

## Usage

Before running the application, set the necessary environment variables in a `.env` file or export them directly in your shell:

```env
FUNDER_PRIVATE_KEY=your_funder_private_key
FUNDING_CONTRACT_ADDRESS=your_funding_contract_address
LOAD_CONTRACT_ADDRESS=your_load_contract_address
RPC_URL=your_rpc_url
```

Run the application with the following command:

```bash
cargo run --release -- -t <tx_count> -n <num_accounts> -f <funding_amount_tssc> -s <set_array_count>
```

- `-t, --tx_count <tx_count>`: The number of transactions to generate.
- `-a, --num_accounts <num_accounts>`: The number of accounts to use for generating transactions.
- `-f, --funding_amount_tssc <funding_amount_tssc>`: The amount of TSSC to fund the accounts with.
- `-s, --set_array_acounts <set_array_count>`: This larger this value, the more each transaction will cost. Reasonable values are 1-1000.
