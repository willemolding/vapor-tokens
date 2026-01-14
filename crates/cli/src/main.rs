use condenser_witness::CondenserWitness;
use transfer_tree::TransferTreeExt;
use vaporize_addresses::generate_vaporize_address;

fn main() {
    let recipient = [0u8; 32];
    let amount = 100u64;

    let mut rng = rand::thread_rng();
    let (addr, secret) = generate_vaporize_address(&mut rng, recipient);

    let mut tree = transfer_tree::TransferTree::<26>::new_empty();
    let proof = tree.append_transfer(recipient, amount);
    let proof_indices = tree.proof_indices(0);
    let root = tree.root();

    let witness = CondenserWitness::builder()
        .recipient(recipient)
        .amount(amount)
        .merkle_root(root)
        .merkle_proof(proof)
        .merkle_proof_indices(proof_indices)
        .vapour_addr(addr)
        .secret(secret)
        .build();

    println!("{}", witness.to_toml());
}
