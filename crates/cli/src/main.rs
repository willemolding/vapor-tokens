use std::str::FromStr;

use ark_bn254::Fr as NoirField;
use clap::Parser;
use condenser_witness::CondenserWitness;
use transfer_tree::TransferTreeExt;
use vaporize_addresses::generate_vaporize_address;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Clone, clap::Subcommand)]
enum Command {
    GenAddress {
        #[arg()]
        recipient: String,
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.cmd {
        Command::GenAddress { recipient } => gen_vapor_address(&recipient),
        Command::BuildWitness {
            vapor_addr,
            amount,
            recipient,
            secret,
        } => build_witness(&vapor_addr, amount, &recipient, &secret),
    }
}

fn gen_vapor_address(recipient: &str) -> anyhow::Result<()> {
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

    Ok(())
}

fn build_witness(
    vapor_addr: &Option<String>,
    amount: u64,
    recipient: &str,
    secret: &Option<String>,
) -> anyhow::Result<()> {
    let (vapor_addr, secret) = if let (Some(vapor_addr), Some(secret)) = (vapor_addr, secret) {
        (
            bs58::decode(vapor_addr)
                .into_vec()?
                .try_into()
                .expect("vapor_addr must be 32 bytes"),
            NoirField::from_str(&secret).expect("vapor_addr must be 32 bytes"),
        )
    } else {
        let mut rng = rand::thread_rng();
        let recipient_bytes: [u8; 32] = bs58::decode(recipient)
            .into_vec()?
            .try_into()
            .expect("recipient must be 32 bytes");
        generate_vaporize_address(&mut rng, recipient_bytes)
    };

    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let mut tree = transfer_tree::TransferTree::<26>::new_empty();
    let proof = tree.append_transfer(vapor_addr, amount);
    let proof_indices = tree.proof_indices(0);
    let root = tree.root();

    let witness = CondenserWitness::builder()
        .recipient(recipient)
        .amount(amount)
        .merkle_root(root)
        .merkle_proof(proof)
        .merkle_proof_indices(proof_indices)
        .vapor_addr(vapor_addr)
        .secret(secret)
        .build();

    println!("{}", witness.to_toml());
    println!();
    println!("Vapor address: {}", bs58::encode(vapor_addr).into_string());
    println!("Recipient: {}", bs58::encode(recipient).into_string());
    Ok(())
}
