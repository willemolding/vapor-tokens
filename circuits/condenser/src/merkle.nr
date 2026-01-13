/// Modifed from zk-kit.noir https://github.com/zk-kit/zk-kit.noir/blob/binary-merkle-root-v0.0.1/packages/binary-merkle-root/src/lib.nr
/// for fixed height trees and using poseidon as the hash function.
use dep::poseidon::poseidon::bn254::hash_2;

pub fn recover_root<let HEIGHT: u32>(
    leaf: Field,
    proof: [Field; HEIGHT],
    proof_indices: [u1; HEIGHT],
) -> Field {
    // Start from the leaf node
    let mut node = leaf;

    // Iterate through the Merkle proof up to HEIGHT
    for i in 0..HEIGHT {
            let sibling = proof[i];

            // Determine the ordering of node and sibling based on the index bit
            // If proof_indices[i] == 0, node is on the left; otherwise, it's on the right
            let (left, right) = if proof_indices[i] == 0 {
                (node, sibling)
            } else {
                (sibling, node)
            };

            // Hash the pair to move one level up the tree
            node = hash_2([left, right]);
        
    }

    // Return the root computed
    node
}