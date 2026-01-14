use ark_bn254::Fr as NoirField;
use ark_ff::{BigInt, BigInteger, PrimeField};
use light_hasher::{Hasher, Poseidon};
use light_sparse_merkle_tree::SparseMerkleTree;

pub type TransferTree<const HEIGHT: usize> = SparseMerkleTree<Poseidon, HEIGHT>;

pub trait TransferTreeExt<const HEIGHT: usize> {
    fn append_transfer(&mut self, recipient: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT];
    fn proof_indices(&self, index: u64) -> [u8; HEIGHT];
}

impl<const HEIGHT: usize> TransferTreeExt<HEIGHT> for TransferTree<HEIGHT> {
    fn append_transfer(&mut self, destination: [u8; 32], amount: u64) -> [[u8; 32]; HEIGHT] {
        let destination = pack_bytes(&destination)
            .iter()
            .map(|f| f.into_bigint().to_bytes_be())
            .collect::<Vec<Vec<u8>>>();

        let leaf =
            Poseidon::hashv(&[&destination[0], &destination[1], &amount.to_be_bytes()]).unwrap();
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

#[inline]
fn fr_from_31_le_bytes(chunk: &[u8; 31]) -> NoirField {
    // 31 bytes = 248 bits, fits comfortably below NoirField modulus, so no reduction issues.
    let mut limbs = [0u64; 4];

    for (i, &b) in chunk.iter().enumerate() {
        let limb = i >> 3; // i / 8
        let shift = (i & 7) << 3; // (i % 8) * 8
        limbs[limb] |= (b as u64) << shift;
    }

    NoirField::from_bigint(BigInt::new(limbs)).expect("31-byte value is < modulus")
}

pub fn pack_bytes(bytes: &[u8]) -> Vec<NoirField> {
    const CHUNK: usize = 31;
    let mut out = Vec::with_capacity((bytes.len() + CHUNK - 1) / CHUNK);

    for c in bytes.chunks(CHUNK) {
        let mut chunk = [0u8; 31];
        chunk[..c.len()].copy_from_slice(c); // zero-pad like Noir's pad_end
        out.push(fr_from_31_le_bytes(&chunk));
    }
    out
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
        let proof = tree.append_transfer(
            [
                24, 190, 156, 60, 238, 7, 189, 235, 169, 222, 217, 179, 62, 139, 220, 233, 237,
                241, 21, 36, 93, 52, 137, 195, 1, 43, 97, 163, 221, 73, 181, 190,
            ],
            2,
        );
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
