use light_compressed_account::hashv_to_bn254_field_size_be;
use light_hasher::Poseidon;
use light_sparse_merkle_tree::SparseMerkleTree;

pub type TransferTree<const HEIGHT: usize> = SparseMerkleTree<Poseidon, HEIGHT>;

pub trait TransferTreeExt<const HEIGHT: usize> {
    fn append_transfer(&mut self, recipient: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT];
}

impl<const HEIGHT: usize> TransferTreeExt<HEIGHT> for TransferTree<HEIGHT> {
    fn append_transfer(&mut self, destination: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT] {
        let leaf = hashv_to_bn254_field_size_be(&[&destination, &amount.to_be_bytes()]);
        self.append(leaf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_transfer() {
        let mut tree = TransferTree::<26>::new_empty();
        let old_root = tree.root();
        println!("Old root: {:?}", old_root);
        tree.append_transfer(
            [
                156, 111, 42, 136, 201, 208, 254, 61, 196, 52, 59, 38, 121, 179, 123, 10, 198, 128,
                127, 2, 60, 4, 128, 66, 91, 226, 221, 37, 137, 211, 217, 175,
            ],
            2,
        );
        let new_root = tree.root();
        println!("New root: {:?}", new_root);
        assert_ne!(old_root, new_root);
    }
}
