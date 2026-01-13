/// Generate a new vaporize address from a recipient address
///
/// Returns the vaporize address and the associated secret value that can be revealed
/// to prove the address is unspendable, and link it to the recipient.
pub fn generate_vaporize_address(recipient: [u8; 32]) -> ([u8; 32], NoirField) {
    todo!()
}
