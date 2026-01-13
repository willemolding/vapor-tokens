use dep::bignum::BigNum;
use dep::bignum::fields::ed25519Fq::ED25519_Fq as Fq;
use dep::poseidon::poseidon::bn254::hash_2;
use ed25519::{assert_is_in_subgroup, assert_is_on_curve, decompress_y};

mod ed25519;

/// Prove knowledge of a secret that links a Vapour address to a recipient address
/// and that the Vapor address is unspendable (i.e. that its x coordinate was derived from a hash and not by multiplying by the generator).
fn main(
    recipient: pub [u8; 32],
    vapour_addr: pub [u8; 32],
    secret: Field,
) {
    // x is derived from poseidon( secret || recipient)
    let x = ed25519_fq_from_noir_field(hash_2([secret, hash_address_to_field(recipient)]));

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
