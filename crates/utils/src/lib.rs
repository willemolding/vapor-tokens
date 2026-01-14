use ark_bn254::Fr as NoirField;
use ark_ff::{BigInt, BigInteger, PrimeField};

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

pub fn unpack_bytes_from_le_fields(fields: &[[u8; 32]], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(fields.len() * 31);

    for b in fields {
        let f = NoirField::from_be_bytes_mod_order(b);
        let mut bytes = f.into_bigint().to_bytes_le();
        bytes.resize(31, 0); // match pack_bytes padding
        out.extend_from_slice(&bytes[..31]);
    }

    out.truncate(len);
    out
}

pub fn fr_to_be_32(f: &NoirField) -> [u8; 32] {
    let bytes = f.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len();
    out[start..].copy_from_slice(&bytes);
    out
}

pub fn u64_to_be_32(x: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&x.to_be_bytes());
    out
}
