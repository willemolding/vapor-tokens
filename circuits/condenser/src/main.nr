use dep::bignum::BigNum;
use dep::bignum::fields::ed25519Fq::ED25519_Fq as Fq;
use dep::poseidon::poseidon::bn254::{hash_3};
use ed25519::{assert_is_in_subgroup, assert_is_on_curve, decompress_y};
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
    let vapor_addr_field = pack_bytes(vapor_addr);
    let leaf = hash_3([vapor_addr_field[0], vapor_addr_field[1], amount]);

    let derived_root = merkle::recover_root(
        leaf,
        merkle_proof,
        merkle_proof_indices,
    );
    assert_eq(merkle_root, derived_root);

    // x is derived from poseidon( recipient || secret )
    let x = ed25519_fq_from_noir_field(hash_3([recipient[0], recipient[1], secret]));

    // y is derived from vapour_addr which is in edwards compressed form
    let y = decompress_y(vapor_addr);

    y.validate_in_field();
    x.validate_in_field();

    assert_is_on_curve(x, y);
    assert_is_in_subgroup(x, y);
}

fn ed25519_fq_from_noir_field(x: Field) -> Fq {
    Fq::from_le_bytes(x.to_le_bytes())
}
