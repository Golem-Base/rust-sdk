use anyhow::Result;
use bigdecimal::BigDecimal;
use clap::{Parser, Subcommand};
use dirs::config_dir;
use golem_base_sdk::{client::GolemBaseClient, Address};
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
    /// Account management commands
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },
    /// Get entity by ID
    GetEntity {
        /// Entity ID to get
        id: String,
    },
    /// Wait until the node is synced
    WaitSync {
        /// Timeout in seconds
        #[arg(short, long, default_value = "120")]
        timeout: u64,
    },
}

#[derive(Subcommand, Debug)]
enum AccountCommand {
    /// List all accounts and their balances
    List,
    /// Create a new account
    Create {
        /// Password for the new account
        #[arg(short, long, default_value = "test123")]
        password: String,
    },
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
}

impl AccountCommand {
    async fn execute(&self, client: &GolemBaseClient) -> Result<()> {
        match self {
            AccountCommand::List => self.handle_list(client).await,
            AccountCommand::Create { password } => self.handle_create(client, password).await,
            AccountCommand::Fund { wallet, amount } => {
                self.handle_fund(client, *wallet, amount.clone()).await
            }
            AccountCommand::Transfer {
                from,
                to,
                amount,
                password,
            } => {
                self.handle_transfer(client, *from, *to, amount.clone(), password)
                    .await
            }
        }
    }

    async fn handle_list(&self, client: &GolemBaseClient) -> Result<()> {
        let accounts = client.account_sync().await?;
        println!("Available accounts:");
        for &addr in &accounts {
            let balance = client.get_balance(addr).await?;
            println!("  {}: {} ETH", addr, balance);
        }
        Ok(())
    }

    async fn handle_create(&self, client: &GolemBaseClient, password: &str) -> Result<()> {
        let account = client.account_generate(password).await?;
        println!("Created new account: {}", account);
        Ok(())
    }

    async fn handle_fund(
        &self,
        client: &GolemBaseClient,
        wallet: Option<Address>,
        amount: BigDecimal,
    ) -> Result<()> {
        let account = if let Some(wallet) = wallet {
            // Load account by address
            client.account_load(wallet, "test123").await?
        } else {
            // Load default private key
            let mut private_key_path =
                config_dir().ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
            private_key_path.push("golembase/private.key");
            client
                .account_load_file(private_key_path, "test123")
                .await?
        };
        println!("Using account: {account:?}");

        let fund_tx = client.fund(account, amount.clone()).await?;
        println!("Account funded with {amount} ETH, transaction hash: {fund_tx:?}");
        Ok(())
    }

    async fn handle_transfer(
        &self,
        client: &GolemBaseClient,
        from: Address,
        to: Address,
        amount: BigDecimal,
        password: &str,
    ) -> Result<()> {
        // Load source account
        let account = client.account_load(from, password).await?;
        println!("Using account: {account:?}");

        // Transfer funds
        let transfer_tx = client.transfer(from, to, amount.clone()).await?;
        println!(
            "Transfer transaction hash for {amount} ETH: {:?}",
            transfer_tx
        );
        Ok(())
    }
}

impl Command {
    async fn execute(&self, client: &GolemBaseClient) -> Result<()> {
        match self {
            Command::Account { command } => command.execute(client).await,
            Command::GetEntity { id } => self.handle_get_entity(client, id).await,
            Command::WaitSync { timeout } => self.handle_wait_sync(client, *timeout).await,
        }
    }

    async fn handle_get_entity(&self, client: &GolemBaseClient, id: &str) -> Result<()> {
        let entry = client.cat(id.parse()?).await?;
        println!("Entry: {}", entry);
        Ok(())
    }

    async fn handle_wait_sync(&self, client: &GolemBaseClient, timeout: u64) -> Result<()> {
        println!("Waiting for node to sync (timeout: {} seconds)...", timeout);
        client
            .sync_node(std::time::Duration::from_secs(timeout))
            .await?;
        println!("Node is synced!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let endpoint = Url::parse(&args.url)?;
    let client = GolemBaseClient::new(endpoint)?;

    // Sync accounts first
    client.account_sync().await?;

    args.command.execute(&client).await
}
