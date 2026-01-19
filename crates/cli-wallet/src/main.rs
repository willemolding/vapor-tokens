use std::str::FromStr;

use crate::borsh_record::BorshRecord;
use borsh::{BorshDeserialize, BorshSerialize};
use clap::Parser;
use redb::TableDefinition;
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};

mod borsh_record;
mod build_merkle_proof;
mod commands;
mod prove;
mod sync;

const TREE_HEIGHT: usize = 26;

/// Table for crated vaporize address metadata indexed by vaporize address.
const VAP_ADDR: TableDefinition<[u8; 32], BorshRecord<VaporAddressRecord>> =
    TableDefinition::new("vapor-addresses");

/// Table for recorded spends index by slot.
/// This is a little bit fraught -- it assumes only one spend per slot which may not hold in reality
const TRANSFERS: TableDefinition<u64, BorshRecord<TransferEvent>> =
    TableDefinition::new("transfers");

#[derive(clap::Parser)]
#[clap(version, about = "CLI wallet for Solana Vapor Tokens", long_about = None)]
struct Args {
    #[clap(subcommand)]
    cmd: Command,

    #[clap(long, env = "SOL_RPC", default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// This is the mint address for the vapor tokens being managed by this wallet
    /// Currently only a single mint is supported per wallet
    #[clap(long, env = "MINT")]
    mint: String,

    /// Path to the wallet database file
    #[clap(long, env = "WALLET_PATH", default_value = "wallet.redb")]
    wallet_file: String,
}

#[derive(Clone, clap::Subcommand)]
enum Command {
    /// Generate a new vaporize address for a recipient address
    /// The recipient address will be hidden within the vaporize address so that unless you know the secret value
    /// it will appear indistinguishable from any other Solana pubkey address
    ///
    /// This will store the generated vaporize address and secret in the local wallet database
    /// The secret is required to spend the funds sent to generated address so keep it safe!
    GenAddress {
        #[clap()]
        recipient: String,
    },
    /// List all vaporize addresses in this wallet along with any deposits made to them
    List,
    /// Condense (mint) vaporized tokens to their destination account
    /// This requires the secret associated with the vaporize address to authorize the mint but this, along with which deposit is being condensed
    /// is hidden within the zero-knowledge proof before being submitted publicly
    Condense {
        #[clap(long, default_value = "~/.config/solana/id.json")]
        keypair: String,

        #[clap()]
        vapor_addr: String,
    },
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize, PartialEq)]
struct VaporAddressRecord {
    addr: [u8; 32],
    recipient: [u8; 32],
    secret: String,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub struct TransferEvent {
    pub to: [u8; 32],
    pub amount: u64,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let db = redb::Database::create(&args.wallet_file)?;

    match args.cmd {
        Command::GenAddress { recipient } => {
            commands::gen_vapor_address(&db, &recipient)?;
        }
        Command::List => {
            sync::sync(&db, &args.rpc_url, &args.mint)?;
            commands::list(&db)?;
        }
        Command::Condense {
            vapor_addr,
            keypair,
        } => {
            let keypair = shellexpand::tilde(&keypair).to_string();
            let signer = read_keypair_file(keypair).unwrap();
            sync::sync(&db, &args.rpc_url, &args.mint)?;
            commands::condense::<TREE_HEIGHT>(
                &db,
                &args.rpc_url,
                signer,
                Pubkey::from_str(&args.mint)?,
                &vapor_addr,
            )?;
        }
    };

    Ok(())
}
