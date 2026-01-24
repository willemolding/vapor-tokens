use dep::bignum::BigNum;
use dep::poseidon::poseidon::bn254::{hash_3};
use ed25519::{assert_is_in_subgroup, assert_is_on_curve, decompress_y, ed25519_fq_from_noir_field};
use nodash::pack_bytes;

mod ed25519;
mod merkle;

global TREE_HEIGHT: u32 = 26;

/// Prove knowledge of a transfer in the transfer tree with the given root
/// and show that the destination is a vapor address which you know the secret and recipient for
/// and that the Vapor address is unspendable (i.e. that its x coordinate was derived from a hash and not by multiplying by the generator).
fn main(
    recipient: pub [Field; 2],
    amount: pub Field,
    merkle_root: pub Field,
    vapor_addr: [u8; 32],
    merkle_proof: [Field; TREE_HEIGHT],
    merkle_proof_indices: [u1; TREE_HEIGHT],
    secret: Field
) {
    assert_valid_merkle_proof(vapor_addr, amount, merkle_root, merkle_proof, merkle_proof_indices);
    assert_is_valid_vapor_address(vapor_addr, recipient, secret);
}

/// Asserts that the given Merkle proof is valid for the given leaf and root
fn assert_valid_merkle_proof(
    vapor_addr: [u8; 32],
    amount: Field,
    merkle_root: Field,
    merkle_proof: [Field; TREE_HEIGHT],
    merkle_proof_indices: [u1; TREE_HEIGHT],
) {
    let vapor_addr_field = pack_bytes(vapor_addr);
    let leaf = hash_3([vapor_addr_field[0], vapor_addr_field[1], amount]);

    let derived_root = merkle::recover_root(
        leaf,
        merkle_proof,
        merkle_proof_indices,
    );
    assert_eq(merkle_root, derived_root);
}

/// Asserts that the given vapor address was generated correctly and lies on the Ed25519 curve in the correct subgroup
fn assert_is_valid_vapor_address(vapor_addr: [u8; 32], recipient: [Field; 2], secret: Field) {
    // x is derived from poseidon( recipient || secret )
    let x = ed25519_fq_from_noir_field(hash_3([recipient[0], recipient[1], secret]));
    let y = decompress_y(vapor_addr);

    y.validate_in_field();
    x.validate_in_field();

    assert_is_on_curve(x, y);
    assert_is_in_subgroup(x, y);
}
