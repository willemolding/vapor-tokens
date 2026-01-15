use std::str::FromStr;

use ark_bn254::Fr as NoirField;
use condenser_witness::CondenserWitness;
use transfer_tree::TransferTreeExt;

pub fn condense(
    db: &redb::Database,
    rpc_url: &str,
    mint: &str,
    vapor_addr: &Option<String>,
) -> anyhow::Result<()> {
    println!("Condense not yet implemented in CLI");
    Ok(())
}

fn build_witness<const HEIGHT: usize>(
    vapor_addr: &str,
    amount: u64,
    recipient: &str,
    secret: &str,
) -> anyhow::Result<CondenserWitness<HEIGHT>> {
    let vapor_addr = bs58::decode(vapor_addr)
        .into_vec()?
        .try_into()
        .expect("vapor_addr must be 32 bytes");
    let secret = NoirField::from_str(secret).expect("secret must be 32 bytes");

    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let mut tree = transfer_tree::TransferTree::<HEIGHT>::new_empty();

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
    Ok(witness)
}
