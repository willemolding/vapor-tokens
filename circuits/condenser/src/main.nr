use dep::bignum::BigNum;
use dep::bignum::fields::ed25519Fq::ED25519_Fq as Fq;
use dep::poseidon::poseidon::bn254::hash_2;
use ed25519::{assert_is_in_subgroup, assert_is_on_curve, decompress_y};
use hash_to_field::hashv_to_bn254_field_size_be_2;

mod ed25519;
mod hash_to_field;
mod merkle;

global TREE_HEIGHT: u32 = 26;

/// Prove knowledge of a transfer in the transfer tree with the given root
/// and show that the destination is a vapor address which you know the secret and recipient for
/// and that the Vapor address is unspendable (i.e. that its x coordinate was derived from a hash and not by multiplying by the generator).
fn main(
    recipient: pub [u8; 32],
    amount: pub Field,
    merkle_root: pub Field,
    vapour_addr: [u8; 32],
    merkle_proof: [Field; TREE_HEIGHT],
    merkle_proof_indices: [u1; TREE_HEIGHT],
    secret: Field
) {
    amount.assert_max_bit_size::<64>();
    let amount_bytes: [u8; 8] = amount.to_be_bytes();
    let leaf = hashv_to_bn254_field_size_be_2(recipient, amount_bytes);

    let derived_root = merkle::recover_root(
        leaf,
        merkle_proof,
        merkle_proof_indices,
    );
    assert_eq(merkle_root, derived_root);

    // x is derived from poseidon( recipient || secret )
    let x = ed25519_fq_from_noir_field(hash_2([hash_address_to_field(recipient), secret]));

    // y is derived from vapour_addr which is in edwards compressed form
    let y = decompress_y(vapour_addr);

    y.validate_in_field();
    x.validate_in_field();

    assert_is_on_curve(x, y);
    assert_is_in_subgroup(x, y);
}

fn ed25519_fq_from_noir_field(x: Field) -> Fq {
    Fq::from_le_bytes(x.to_le_bytes())
}

fn le_bytes16_to_field(bytes: [u8; 16]) -> Field {
    let mut acc: Field = 0;
    let mut base: Field = 1;
    for i in 0..16 {
        acc = acc + base * (bytes[i] as Field);
        base = base * (256 as Field);
    }
    acc
}

fn hash_address_to_field(pk_bytes: [u8; 32]) -> Field {
    let mut hi: [u8; 16] = [0; 16];
    let mut lo: [u8; 16] = [0; 16];

    for i in 0..16 {
        hi[i] = pk_bytes[i];
        lo[i] = pk_bytes[i + 16];
    }

    let a: Field = le_bytes16_to_field(hi);
    let b: Field = le_bytes16_to_field(lo);

    hash_2([a, b])
}
