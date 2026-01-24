use dep::bignum::BigNum;
use dep::bignum::fields::ed25519Fq::ED25519_Fq as Fq;

// Ed25519 twisted Edwards parameter d
// See https://www.johndcook.com/blog/2025/07/27/moneros-elliptic-curve/
pub global EDWARDS_D: Fq = Fq::from_limbs([
    581746193016579820166537227703777443,
    565598715678867512207437505225070592,
    20995,
]);

/// Does not revert iff (x, y) lies on the Ed25519 curve
/// (Assumes x, y are in field)
pub fn assert_is_on_curve(x: Fq, y: Fq) {
    let x2 = x * x;
    let y2 = y * y;

    // lhs = y^2 - x^2
    let lhs = y2 - x2;

    // rhs = 1 + d * x^2 * y^2
    let rhs = Fq::one() + EDWARDS_D * x2 * y2;

    assert(lhs == rhs)
}

// Reverts if [8]P == identity
// This is a standard subgroup check for Ed25519
// (Assumes on-curve already.)
pub fn assert_is_in_subgroup(x: Fq, y: Fq) {
    let mut p = ExtPoint::from((x, y));
    p = p.dbl(); // [2]P
    p = p.dbl(); // [4]P
    p = p.dbl(); // [8]P

    assert_not_identity(p);
}

pub fn decompress_y(pk_bytes: [u8; 32]) -> Fq {
    // Clear sign bit from y encoding
    let mut y_bytes = pk_bytes;
    y_bytes[31] = pk_bytes[31] & 0x7F;

    Fq::from_le_bytes(y_bytes)
}

pub fn ed25519_fq_from_noir_field(x: Field) -> Fq {
    Fq::from_le_bytes(x.to_le_bytes())
}

// Extended Edwards coordinates (X:Y:Z:T)
struct ExtPoint {
    X: Fq,
    Y: Fq,
    Z: Fq,
    T: Fq,
}

impl ExtPoint {
    // Extended-coordinate doubling (complete for twisted Edwards; no inversion)
    fn dbl(self) -> ExtPoint {
        let a = -Fq::one();

        let A = self.X * self.X;
        let B = self.Y * self.Y;
        let C = Fq::from(2) * (self.Z * self.Z);
        let D = a * A;
        let E = (self.X + self.Y) * (self.X + self.Y) - A - B;
        let G = D + B;
        let F = G - C;
        let H = D - B;

        ExtPoint { X: E * F, Y: G * H, Z: F * G, T: E * H }
    }
}

impl From<(Fq, Fq)> for ExtPoint {
    fn from(p: (Fq, Fq)) -> ExtPoint {
        ExtPoint { X: p.0, Y: p.1, Z: Fq::one(), T: p.0 * p.1 }
    }
}

fn assert_not_identity(p: ExtPoint) {
    assert((p.X != Fq::zero()) | (p.Y != p.Z));
}
