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
