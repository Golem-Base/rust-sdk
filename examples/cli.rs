use alloy::primitives::Address;
use anyhow::Result;
use bigdecimal::BigDecimal;
use clap::{Parser, Subcommand};
use dirs::config_dir;
use golem_base_sdk::client::GolemBaseClient;
use url::Url;

/// Program to fund and transfer funds between accounts on Golem Base
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL of the GolemBase node
    #[arg(short, long, default_value = "http://localhost:8545")]
    url: String,

    /// Command to execute
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List all accounts and their balances
    List,
    /// Fund an account with ETH
    Fund {
        /// Address of the wallet to fund (optional, uses default private key if not specified)
        #[arg(short, long)]
        wallet: Option<Address>,

        /// Amount in ETH to fund
        #[arg(short, long, default_value = "1.0")]
        amount: BigDecimal,
    },
    /// Transfer ETH to another account
    Transfer {
        /// Address of the source wallet
        #[arg(short, long)]
        from: Address,

        /// Address of the destination wallet
        #[arg(short, long)]
        to: Address,

        /// Amount in ETH to transfer
        #[arg(short, long)]
        amount: BigDecimal,

        /// Password for the source wallet
        #[arg(short, long, default_value = "test123")]
        password: String,
    },
    /// Get entity by ID
    GetEntity {
        /// Entity ID to get
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let endpoint = Url::parse(&args.url)?;
    let client = GolemBaseClient::new(endpoint)?;

    // Sync accounts first
    let accounts = client.account_sync().await?;

    match args.command {
        Command::List => {
            log::info!("Available accounts:");
            for &addr in &accounts {
                let balance = client.get_balance(addr).await?;
                log::info!("  {}: {} ETH", addr, balance);
            }
        }
        Command::Fund { wallet, amount } => {
            let account = if let Some(wallet) = wallet {
                // Load account by address
                client.account_load(wallet, "test123").await?
            } else {
                // Load default private key
                let mut private_key_path = config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
                private_key_path.push("golembase/private.key");
                client
                    .account_load_file(private_key_path, "test123")
                    .await?
            };
            log::info!("Using account: {account:?}");

            let fund_tx = client.fund(account, amount.clone()).await?;
            log::info!("Account funded with {amount} ETH, transaction hash: {fund_tx:?}");
        }
        Command::Transfer {
            from,
            to,
            amount,
            password,
        } => {
            // Load source account
            let account = client.account_load(from, &password).await?;
            log::info!("Using account: {account:?}");

            // Transfer funds
            let transfer_tx = client.transfer(from, to, amount.clone()).await?;
            log::info!(
                "Transfer transaction hash for {amount} ETH: {:?}",
                transfer_tx
            );
        }
        Command::GetEntity { id } => {
            let entry = client.cat(id).await?;
            println!("Entry: {}", entry);
        }
    }

    Ok(())
}
