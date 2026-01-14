use ark_bn254::Fr as NoirField;
use ark_ff::{AdditiveGroup, BigInteger, One, PrimeField};
use light_hasher::Poseidon;
use light_poseidon::{Poseidon as FieldPoseidon, PoseidonHasher};
use light_sparse_merkle_tree::SparseMerkleTree;

pub type TransferTree<const HEIGHT: usize> = SparseMerkleTree<Poseidon, HEIGHT>;

pub trait TransferTreeExt<const HEIGHT: usize> {
    fn append_transfer(&mut self, recipient: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT];
    fn proof_indices(&self, index: u64) -> [u8; HEIGHT];
}

impl<const HEIGHT: usize> TransferTreeExt<HEIGHT> for TransferTree<HEIGHT> {
    fn append_transfer(&mut self, destination: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT] {
        let mut h = FieldPoseidon::<NoirField>::new_circom(2).unwrap();
        let destination = pack_bytes(&destination);

        let dest_hashed = h.hash(&destination).unwrap();
        let leaf = h.hash(&[dest_hashed, NoirField::from(amount)]).unwrap();

        self.append(leaf.into_bigint().to_bytes_be().try_into().unwrap())
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

fn pack_bytes(bytes: &[u8]) -> Vec<NoirField> {
    const CHUNK: usize = 31;
    bytes
        .chunks(CHUNK)
        .map(|chunk| {
            let mut acc = NoirField::ZERO;
            let mut base = NoirField::one();

            for &b in chunk {
                acc += base * NoirField::from(b as u64);
                base *= NoirField::from(256u64);
            }

            acc
        })
        .collect()
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
        let proof = tree.append_transfer([255; 32], 2);
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
