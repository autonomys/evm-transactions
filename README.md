# EVM Transaction Generator

EVM Transaction Generator is a Rust-based command-line application that automates the process of generating and sending transactions to an Ethereum Virtual Machine (EVM) compatible blockchain.

## Features

- Generate a specified number of transactions.
- Automatically handle Ethereum wallet creation and transaction signing.
- Configurable via command-line arguments.
- Error handling with detailed logging.

## Requirements

- Rust Programming Language
- Ethereum node accessible via an HTTP endpoint

## Installation

Clone the repository and build the project:

```bash
git clone https://github.com/jfrank-summit/evm-tx.git
cd evm-tx-generator
cargo build --release
```

## Usage

Run the program with the following command:

```bash
cargo run -- -n <NODE_URL> -t <TX_COUNT> -k <PRIVATE_KEY>
```

- `-n`, `--node_url`: URL of the Ethereum node.
- `-t`, `--tx_count`: Number of transactions to generate.
- `-k`, `--private_key`: Private key for the Ethereum wallet.

## Logging

Logging is set up to provide insights into the application's process and errors. By default, the log level is set to `Info`. To change the log level, set the `RUST_LOG` environment variable.

## Example

```bash
RUST_LOG=debug cargo run -- -n 'https://ethereum-node-url.com' -t 10 -k 'your-private-key'
```

## Contribution

Contributions are welcome. Please feel free to submit pull requests or open issues.

## License

[Your chosen license]

## Disclaimer

This tool is for educational and development purposes only. Do not use it on mainnet with real funds without thorough testing and understanding of the risks.