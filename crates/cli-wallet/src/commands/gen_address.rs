use vaporize_addresses::generate_vaporize_address;

use crate::{VAP_ADDR, VaporAddressRecord};

pub(crate) fn gen_vapor_address(db: &redb::Database, recipient: &str) -> anyhow::Result<()> {
    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let mut rng = rand::thread_rng();
    let (addr, secret) = generate_vaporize_address(&mut rng, recipient);
    let address = bs58::encode(addr).into_string();

    println!("Generated vaporize address: {}", address);
    qr2term::print_qr(address)?;
    println!("");
    println!("Spend secret: {}", secret.to_string());

    let record = VaporAddressRecord {
        addr,
        recipient,
        secret: secret.to_string(),
    };

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(VAP_ADDR)?;

        table.insert(&record.addr, &record)?;
    }
    write_txn.commit()?;

    Ok(())
}
