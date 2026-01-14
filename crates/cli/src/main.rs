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
        vapor_addr: String,

        #[arg(long)]
        amount: u64,

        #[arg(long)]
        recipient: String,

        #[arg(long)]
        secret: String,
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
        } => build_witness(&vapor_addr, amount, &recipient, secret),
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
    vapor_addr: &str,
    amount: u64,
    recipient: &str,
    secret: String,
) -> anyhow::Result<()> {
    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let vapour_addr: [u8; 32] = bs58::decode(vapor_addr)
        .into_vec()?
        .try_into()
        .expect("vapor_addr must be 32 bytes");

    let secret = NoirField::from_str(&secret).expect("vapor_addr must be 32 bytes");

    let mut tree = transfer_tree::TransferTree::<26>::new_empty();
    let proof = tree.append_transfer(vapour_addr, amount);
    let proof_indices = tree.proof_indices(0);
    let root = tree.root();

    let witness = CondenserWitness::builder()
        .recipient(recipient)
        .amount(amount)
        .merkle_root(root)
        .merkle_proof(proof)
        .merkle_proof_indices(proof_indices)
        .vapour_addr(vapour_addr)
        .secret(secret)
        .build();

    println!("{}", witness.to_toml());
    Ok(())
}
