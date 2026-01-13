use rand::rngs::OsRng;
use vaporize_addresses::generate_vaporize_address;

fn main() -> anyhow::Result<()> {
    let mut rng = OsRng;

    let recipient = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Expected recipient address as input"))?;
    let recipient = bs58::decode(recipient).into_vec()?;

    let (address, secret) = generate_vaporize_address(
        &mut rng,
        recipient
            .try_into()
            .map_err(|_| anyhow::anyhow!("Recipient address must be exactly 32 bytes"))?,
    );

    let address = bs58::encode(address).into_string();

    println!("Generated vaporize address: {}", address);
    qr2term::print_qr(address)?;
    println!("");
    println!("Spend secret: {}", secret.to_string());

    Ok(())
}
