use ark_bn254::Fr as NoirField;
use ark_ff::PrimeField;
use utils::pack_bytes;

use bon::bon;

#[derive(Clone, Debug)]
pub struct CondenserWitness<const HEIGHT: usize> {
    pub recipient: [NoirField; 2],
    pub amount: NoirField,
    pub merkle_root: NoirField,
    pub vapor_addr: [u8; 32],
    pub merkle_proof: [NoirField; HEIGHT],
    pub merkle_proof_indices: [u8; HEIGHT],
    pub secret: NoirField,
}

#[bon]
impl<const HEIGHT: usize> CondenserWitness<HEIGHT> {
    #[builder]
    pub fn new(
        recipient: [u8; 32],
        amount: u64,
        merkle_root: [u8; 32],
        vapor_addr: [u8; 32],
        merkle_proof: [[u8; 32]; HEIGHT],
        merkle_proof_indices: [u8; HEIGHT],
        secret: NoirField,
    ) -> Self {
        Self {
            recipient: pack_bytes(&recipient)
                .try_into()
                .expect("recipient must be 2 field elements"),
            amount: NoirField::from(amount),
            merkle_root: NoirField::from_be_bytes_mod_order(&merkle_root),
            vapor_addr,
            merkle_proof: merkle_proof.map(|node| NoirField::from_be_bytes_mod_order(&node)),
            merkle_proof_indices,
            secret,
        }
    }

    pub fn to_toml(&self) -> String {
        let mut toml_str = String::new();

        toml_str.push_str(&format!(
            "recipient = {:?}\n",
            self.recipient
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
        ));
        toml_str.push_str(&format!("amount = \"{}\"\n", self.amount));

        toml_str.push_str(&format!(
            "merkle_root = \"{}\"\n",
            self.merkle_root.to_string()
        ));

        toml_str.push_str(&format!(
            "vapor_addr = {:?}\n",
            self.vapor_addr
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
        ));

        toml_str.push_str(&format!(
            "merkle_proof = {:?}\n",
            self.merkle_proof
                .iter()
                .map(|node| node.to_string())
                .collect::<Vec<_>>()
        ));

        toml_str.push_str(&format!(
            "merkle_proof_indices = {:?}\n",
            self.merkle_proof_indices
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
        ));

        toml_str.push_str(&format!("secret = {:?}\n", self.secret.to_string()));

        toml_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_toml() {
        let witness = CondenserWitness {
            recipient: [NoirField::from(1u64); 2],
            amount: NoirField::from(42u64),
            merkle_root: NoirField::from(2u64),
            vapor_addr: [3u8; 32],
            merkle_proof: [NoirField::from(0u64); 26],
            merkle_proof_indices: [0u8; 26],
            secret: NoirField::from(4u64),
        };
        let toml_output = witness.to_toml();
        println!("{}", toml_output);
    }
}
