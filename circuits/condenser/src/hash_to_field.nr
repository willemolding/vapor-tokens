use dep::keccak256::keccak256;

global HASH_TO_FIELD_SIZE_SEED: u8 = 255;

/// Mirrors light-hasher hashv_to_bn254_field_size_be:
/// - Keccak256 over concatenated slices + bump seed (0xFF)
/// - Zero the most-significant byte to stay within BN254 field size
pub fn hashv_to_bn254_field_size_be<let N: u32>(bytes: [u8; N]) -> Field {
    let mut input: [u8; N + 1] = [0; N + 1];
    for i in 0..N {
        input[i] = bytes[i];
    }
    input[N] = HASH_TO_FIELD_SIZE_SEED;

    let mut hashed = keccak256(input, input.len());
    hashed[0] = 0;
    Field::from_be_bytes(hashed)
}

pub fn hashv_to_bn254_field_size_be_2<let A: u32, let B: u32>(
    a: [u8; A],
    b: [u8; B],
) -> Field {
    let mut combined: [u8; A + B] = [0; A + B];
    for i in 0..A {
        combined[i] = a[i];
    }
    for i in 0..B {
        combined[A + i] = b[i];
    }
    hashv_to_bn254_field_size_be(combined)
}
