use ark_bn254::Fr as NoirField;
use ark_ec::twisted_edwards::TECurveConfig;
use ark_ed25519::{EdwardsAffine, EdwardsConfig, Fq};
use ark_ff::{Field, One, PrimeField, UniformRand, Zero};
use ark_serialize::CanonicalSerialize;
use light_poseidon::{Poseidon, PoseidonHasher};
use rand::Rng;

/// Given a recipient address (32 bytes) generate an unspendable vapourize address
/// and return it plus the secret value required to prove ownership
pub fn generate_vaporize_address<R: rand::RngCore>(
    rng: &mut R,
    recipient: [u8; 32],
) -> ([u8; 32], NoirField) {
    // Try random values until we find a valid point on the ed25519 curve
    // This should take on average 2 tries
    let (p, r) = loop {
        let r = NoirField::rand(rng);
        let x = hash_2(hash_address_to_field(recipient), r);
        if let Some(p) = ed25519_point_from_x(rng, ed25519_fq_from_noir_field(&x)) {
            break (p, r);
        }
    };

    let mut addr = [0u8; 32];
    p.serialize_compressed(&mut addr[..]).unwrap();
    (addr, r)
}

fn hash_2(a: NoirField, b: NoirField) -> NoirField {
    let mut poseidon = Poseidon::<NoirField>::new_circom(2).unwrap();
    poseidon.hash(&[a, b]).unwrap()
}

fn ed25519_fq_from_noir_field(x: &NoirField) -> Fq {
    Fq::from_bigint(x.into_bigint()).unwrap()
}

/// Split a [u8;32] into two halves, interpret each half as a field element
/// and hash them together using Poseidon
fn hash_address_to_field(x: [u8; 32]) -> NoirField {
    let mut hi = [0u8; 16];
    let mut lo = [0u8; 16];
    hi.copy_from_slice(&x[0..16]);
    lo.copy_from_slice(&x[16..32]);

    let a = NoirField::from_le_bytes_mod_order(&hi);
    let b = NoirField::from_le_bytes_mod_order(&lo);

    let mut poseidon = Poseidon::<NoirField>::new_circom(2).unwrap();
    poseidon.hash(&[a, b]).unwrap()
}

/// Given an x coordinate, attempt to find the corresponding y coordinate on the ed25519 curve
/// Note that both the positive and negative y coordinates are valid, this will randomly pick one
fn ed25519_point_from_x<R: rand::RngCore>(rng: &mut R, x: Fq) -> Option<EdwardsAffine> {
    let a = EdwardsConfig::COEFF_A;
    let d = EdwardsConfig::COEFF_D;

    // evaluate  y^2 = (1 - a*x^2) / (1 - d*x^2)
    // and bail if there is no solution for y
    let x2 = x.square();
    let num = Fq::one() - (a * x2);
    let den = Fq::one() - (d * x2);
    if den.is_zero() {
        return None;
    }
    let y2 = num * den.inverse()?;
    let y = y2.sqrt()?;

    // randomly pick +y or -y
    let y = if rng.gen_bool(0.5) { y } else { -y };

    // verify that the point is in the correct subgroup
    let point = EdwardsAffine::new_unchecked(x, y);
    if !point.is_in_correct_subgroup_assuming_on_curve() {
        return None;
    }

    Some(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curve25519_dalek::edwards::CompressedEdwardsY;
    use rand::rngs::OsRng;

    #[test]
    fn test_generate_vaporize_address() {
        // fuzz a bunch of recipients and check we get valid addresses
        for i in 0..255 {
            let mut rng = OsRng;
            let (addr, _secret) = generate_vaporize_address(&mut rng, [i; 32]);
            println!("Generated address: {:?}", addr);

            // check that decompressing works
            CompressedEdwardsY(addr)
                .decompress()
                .expect("not a valid point encoding");
        }
    }
}
