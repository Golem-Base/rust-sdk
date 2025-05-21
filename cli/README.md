# Golem Base CLI

A command-line interface tool for interacting with Golem Base.

## Installation

```bash
cargo install --path .
```

## Usage

To see detailed debug logs, set the `RUST_LOG` environment variable before runnign any command:
```bash
export RUST_LOG=debug
golem-base-sdk-cli <command>
```

The CLI provides several commands for managing accounts and transactions:

### List all accounts and their balances
```bash
golem-base-sdk-cli list
```

### Fund an account
```bash
golem-base-sdk-cli fund [--wallet <WALLET>] [--amount <AMOUNT>]
```

### Transfer ETH between accounts
```bash
golem-base-sdk-cli transfer --from <FROM> --to <TO> --amount <AMOUNT> [--password <PASSWORD>]
```

### Get entity by ID
```bash
golem-base-sdk-cli get-entity <ID>
```

## Configuration

The CLI uses the system's config directory to store account information. On Linux, this is typically `~/.config/golembase/`.


## Development

To build the project:
```bash
cargo build
```

To run tests:
```bash
cargo test
```
