use light_compressed_account::hashv_to_bn254_field_size_be;
use light_hasher::Poseidon;
use light_sparse_merkle_tree::SparseMerkleTree;

pub type TransferTree<const HEIGHT: usize> = SparseMerkleTree<Poseidon, HEIGHT>;

pub trait TransferTreeExt<const HEIGHT: usize> {
    fn append_transfer(&mut self, recipient: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT];
    fn proof_indices(&self, index: u64) -> [u8; HEIGHT];
}

impl<const HEIGHT: usize> TransferTreeExt<HEIGHT> for TransferTree<HEIGHT> {
    fn append_transfer(&mut self, destination: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT] {
        let leaf = hashv_to_bn254_field_size_be(&[&destination, &amount.to_be_bytes()]);
        self.append(leaf)
    }

    fn proof_indices(&self, index: u64) -> [u8; HEIGHT] {
        let mut indices = [0u8; HEIGHT];
        let mut idx = index;
        for i in 0..HEIGHT {
            indices[i] = (idx & 1) as u8;
            idx >>= 1;
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn test_append_transfer() {
        let mut tree = TransferTree::<26>::new_empty();
        let old_root = tree.root();
        let old_root_int = BigUint::from_bytes_be(&old_root);
        println!("Old root: {}", old_root_int);
        let proof = tree.append_transfer(
            [
                156, 111, 42, 136, 201, 208, 254, 61, 196, 52, 59, 38, 121, 179, 123, 10, 198, 128,
                127, 2, 60, 4, 128, 66, 91, 226, 221, 37, 137, 211, 217, 175,
            ],
            2,
        );
        let new_root = tree.root();
        println!(
            "New root: {:?}",
            BigUint::from_bytes_be(&new_root).to_string()
        );
        assert_ne!(old_root, new_root);

        println!(
            "Proof: {:?}",
            proof
                .iter()
                .map(|b| BigUint::from_bytes_be(b).to_string())
                .collect::<Vec<_>>()
        );
        println!("Proof indices: {:?}", tree.proof_indices(0));
    }
}
