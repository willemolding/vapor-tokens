use std::{path::PathBuf, str::FromStr};

use crate::borsh_record::BorshRecord;
use ark_bn254::Fr as NoirField;
use ark_ff::{BigInteger, PrimeField};
use borsh::{BorshDeserialize, BorshSerialize};
use clap::{CommandFactory, Parser};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_cli_config::Config;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, read_keypair_file},
};
use vaporize_addresses::generate_vaporize_address;

mod borsh_record;
mod condense;
mod prove;
mod sync;

const TREE_HEIGHT: usize = 26;

const VAP_ADDR: TableDefinition<[u8; 32], BorshRecord<VaporAddressRecord>> =
    TableDefinition::new("vapor-addresses");

/// Table for recorded spends index by slot.
/// This is a little bit fraught -- it assumes only one spend per slot which may not hold in reality
const TRANSFERS: TableDefinition<u64, BorshRecord<TransferEvent>> =
    TableDefinition::new("transfers");

#[derive(clap::Parser)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    cmd: Command,

    #[clap(long, env = "SOL_RPC", default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[clap(long, env = "MINT")]
    mint: String,

    #[clap(long, env = "WALLET_PATH", default_value = "wallet.redb")]
    wallet_file: String,
}

#[derive(Clone, clap::Subcommand)]
enum Command {
    GenAddress {
        #[clap()]
        recipient: String,
    },
    List,
    Condense {
        #[clap(long)]
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
            gen_vapor_address(&db, &recipient)?;
        }
        Command::List => {
            sync::sync(&db, &args.rpc_url, &args.mint)?;
            list(&db)?;
        }
        Command::Condense {
            vapor_addr,
            keypair,
        } => {
            let signer = read_keypair_file(keypair).unwrap();
            sync::sync(&db, &args.rpc_url, &args.mint)?;
            condense::condense::<TREE_HEIGHT>(
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

fn gen_vapor_address(db: &redb::Database, recipient: &str) -> anyhow::Result<()> {
    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let mut rng = rand::thread_rng();
    let (addr, secret) = generate_vaporize_address(&mut rng, recipient);
    let address = bs58::encode(addr).into_string();

    println!("Generated vaporize address: {}", address);
    qr2term::print_qr(address)?;
    println!("");
    println!("Spend secret: {}", secret.to_string());

    let record = VaporAddressRecord {
        addr,
        recipient,
        secret: secret.to_string(),
    };

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(VAP_ADDR)?;

        table.insert(&record.addr, &record)?;
    }
    write_txn.commit()?;

    Ok(())
}

fn list(db: &redb::Database) -> anyhow::Result<()> {
    let read_txn = db.begin_read()?;
    {
        let addresses = read_txn.open_table(VAP_ADDR)?;
        let transfers = read_txn.open_table(TRANSFERS)?;

        for result in addresses.iter()? {
            let (key, record) = result?;
            let address = bs58::encode(key.value()).into_string();
            let recipient = bs58::encode(record.value().recipient).into_string();
            let secret = record.value().secret;

            println!("Vaporize Address: {}", address);
            println!("  Recipient: {}", recipient);
            println!("  Secret: {}", secret);
            println!("  Deposits:");
            for result in transfers.iter()? {
                let (slot_key, transfer) = result?;
                if transfer.value().to == key.value() {
                    println!(
                        "    Received {} in slot {}",
                        transfer.value().amount,
                        slot_key.value()
                    );
                }
            }
            println!();
        }
    }
    Ok(())
}
