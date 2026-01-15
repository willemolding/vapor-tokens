use std::str::FromStr;

use ark_bn254::Fr as NoirField;
use ark_ff::{BigInteger, PrimeField};
use borsh::{BorshDeserialize, BorshSerialize};
use clap::Parser;
use condenser_witness::CondenserWitness;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use transfer_tree::TransferTreeExt;
use vaporize_addresses::generate_vaporize_address;

use crate::borsh_record::BorshRecord;

mod borsh_record;
mod condense;
mod sync;

const VAP_ADDR: TableDefinition<[u8; 32], BorshRecord<VaporAddressRecord>> =
    TableDefinition::new("vapor-addresses");

/// Table for recorded spends index by slot.
/// This is a little bit fraught -- it assumes only one spend per slot which may not hold in reality
const TRANSFERS: TableDefinition<u64, BorshRecord<TransferEvent>> =
    TableDefinition::new("transfers");

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,

    #[arg(long, env = "SOL_RPC", default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[arg(long, env = "MINT")]
    mint: String,
}

#[derive(Clone, clap::Subcommand)]
enum Command {
    GenAddress {
        #[arg()]
        recipient: String,
    },
    List,
    Sync,
    Condense {
        #[arg(long)]
        vapor_addr: Option<String>,
    },
    BuildWitness {
        #[arg(long)]
        vapor_addr: Option<String>,

        #[arg(long)]
        amount: u64,

        #[arg(long)]
        recipient: String,

        #[arg(long)]
        secret: Option<String>,
    },
}

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
struct VaporAddressRecord {
    addr: [u8; 32],
    recipient: [u8; 32],
    secret: [u8; 32],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub struct TransferEvent {
    pub to: [u8; 32],
    pub amount: u64,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db = redb::Database::create("wallet.redb")?;
    let args = Args::parse();

    match args.cmd {
        Command::GenAddress { recipient } => gen_vapor_address(&db, &recipient),
        Command::List => list(&db),
        Command::Sync => sync::sync(&db, &args.rpc_url, &args.mint),
        Command::Condense { vapor_addr } => {
            condense::condense(&db, &args.rpc_url, &args.mint, &vapor_addr)
        }
        Command::BuildWitness {
            vapor_addr,
            amount,
            recipient,
            secret,
        } => build_witness(&vapor_addr, amount, &recipient, &secret),
    }
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

    let secret_bytes: [u8; 32] = secret.into_bigint().to_bytes_be().try_into().unwrap();
    let record = VaporAddressRecord {
        addr,
        recipient,
        secret: secret_bytes,
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
            let secret = NoirField::from_be_bytes_mod_order(&record.value().secret).to_string();

            println!("Vaporize Address: {}", address);
            println!("  Recipient: {}", recipient);
            println!("  Spend Secret: {}", secret);
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

fn build_witness(
    vapor_addr: &Option<String>,
    amount: u64,
    recipient: &str,
    secret: &Option<String>,
) -> anyhow::Result<()> {
    // let (vapor_addr, secret) = if let (Some(vapor_addr), Some(secret)) = (vapor_addr, secret) {
    //     (
    //         bs58::decode(vapor_addr)
    //             .into_vec()?
    //             .try_into()
    //             .expect("vapor_addr must be 32 bytes"),
    //         NoirField::from_str(&secret).expect("vapor_addr must be 32 bytes"),
    //     )
    // } else {
    //     let mut rng = rand::thread_rng();
    //     let recipient_bytes: [u8; 32] = bs58::decode(recipient)
    //         .into_vec()?
    //         .try_into()
    //         .expect("recipient must be 32 bytes");
    //     generate_vaporize_address(&mut rng, recipient_bytes)
    // };

    // let recipient: [u8; 32] = bs58::decode(recipient)
    //     .into_vec()?
    //     .try_into()
    //     .expect("recipient must be 32 bytes");

    // let mut tree = transfer_tree::TransferTree::<26>::new_empty();
    // let proof = tree.append_transfer(vapor_addr, amount);
    // let proof_indices = tree.proof_indices(0);
    // let root = tree.root();

    // let witness = CondenserWitness::builder()
    //     .recipient(recipient)
    //     .amount(amount)
    //     .merkle_root(root)
    //     .merkle_proof(proof)
    //     .merkle_proof_indices(proof_indices)
    //     .vapor_addr(vapor_addr)
    //     .secret(secret)
    //     .build();

    // println!("{}", witness.to_toml());
    // println!();
    // println!("Vapor address: {}", bs58::encode(vapor_addr).into_string());
    // println!("Recipient: {}", bs58::encode(recipient).into_string());
    Ok(())
}
