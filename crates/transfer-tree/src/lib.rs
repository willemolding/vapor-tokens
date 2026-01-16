use ark_ff::{BigInteger, PrimeField};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{ConcurrentMerkleTree, errors::ConcurrentMerkleTreeError};
use light_hasher::{Hasher, Poseidon};
use utils::pack_bytes;

pub type TransferTree<const HEIGHT: usize> = ConcurrentMerkleTree<Poseidon, HEIGHT>;

pub trait TransferTreeExt<const HEIGHT: usize> {
    fn new_empty() -> TransferTree<HEIGHT>;

    fn append_transfer(
        &mut self,
        destination: [u8; 32],
        amount: u64,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError>;

    fn append_transfer_with_proof(
        &mut self,
        recipient: [u8; 32],
        amount: u64,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError>;

    fn validate_transfer_proof(
        &self,
        destination: [u8; 32],
        amount: u64,
        index: usize,
        proof: &BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError>;

    fn proof_indices(&self, index: usize) -> [u8; HEIGHT];
}

impl<const HEIGHT: usize> TransferTreeExt<HEIGHT> for TransferTree<HEIGHT> {
    fn new_empty() -> TransferTree<HEIGHT> {
        let mut tree = TransferTree::<HEIGHT>::new(HEIGHT, 2, 1, 0).unwrap();
        tree.init().unwrap();
        tree
    }

    fn append_transfer(
        &mut self,
        destination: [u8; 32],
        amount: u64,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        let leaf = build_leaf(destination, amount);
        self.append(&leaf)
    }

    fn append_transfer_with_proof(
        &mut self,
        destination: [u8; 32],
        amount: u64,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        let leaf = build_leaf(destination, amount);
        self.append_with_proof(&leaf, proof)
    }

    fn validate_transfer_proof(
        &self,
        destination: [u8; 32],
        amount: u64,
        index: usize,
        proof: &BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let leaf = build_leaf(destination, amount);
        self.validate_proof(&leaf, index, proof)
    }

    fn proof_indices(&self, index: usize) -> [u8; HEIGHT] {
        let mut indices = [0u8; HEIGHT];
        let mut idx = index;
        for i in 0..HEIGHT {
            indices[i] = (idx & 1) as u8;
            idx >>= 1;
        }
        indices
    }
}

fn build_leaf(destination: [u8; 32], amount: u64) -> [u8; 32] {
    let destination_bytes = pack_bytes(&destination)
        .iter()
        .map(|f| f.into_bigint().to_bytes_be())
        .collect::<Vec<Vec<u8>>>();

    Poseidon::hashv(&[
        &destination_bytes[0],
        &destination_bytes[1],
        &amount.to_be_bytes(),
    ])
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn test_append_transfer() {
        let mut tree = TransferTree::<26>::new_empty();

        let old_root = tree.root();
        println!("Old root: {:?}", old_root);

        let mut proof = BoundedVec::with_capacity(26);
        let leaf_index = tree.next_index();
        let (mut changelog_index, _sequence_number) = tree
            .append_transfer_with_proof(
                [
                    24, 190, 156, 60, 238, 7, 189, 235, 169, 222, 217, 179, 62, 139, 220, 233, 237,
                    241, 21, 36, 93, 52, 137, 195, 1, 43, 97, 163, 221, 73, 181, 190,
                ],
                2,
                &mut proof,
            )
            .unwrap();

        // append a new transfer and update prior proof
        tree.append_transfer(
            [
                24, 190, 156, 60, 238, 7, 189, 235, 169, 222, 217, 179, 62, 139, 220, 233, 237,
                241, 21, 36, 93, 52, 137, 195, 1, 43, 97, 163, 221, 73, 181, 190,
            ],
            10,
        )
        .unwrap();
        if changelog_index != tree.changelog_index() {
            tree.update_proof_from_changelog(changelog_index, leaf_index, &mut proof)
                .unwrap();
            changelog_index = tree.changelog_index();
        }

        let new_root = tree.root();
        println!("New root: {:?}", new_root);
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
