use redb::{ReadableDatabase, ReadableTable};

use crate::{TRANSFERS, VAP_ADDR};

pub(crate) fn list(db: &redb::Database) -> anyhow::Result<()> {
    let read_txn = db.begin_read()?;
    {
        let addresses = read_txn.open_table(VAP_ADDR)?;
        let transfers = read_txn.open_table(TRANSFERS)?;

        for result in addresses.iter()? {
            let (key, record) = result?;
            let address = bs58::encode(key.value()).into_string();
            let recipient = bs58::encode(record.value().recipient).into_string();
            let secret = record.value().secret;

            println!("Vaporize Address: {}", address);
            println!("  Recipient: {}", recipient);
            println!("  Secret: {}", secret);
            println!("  Deposits:");
            for result in transfers.iter()? {
                let (slot_key, transfer) = result?;
                if transfer.value().to == key.value() {
                    println!(
                        "    Received {} in slot {}",
                        transfer.value().amount,
                        slot_key.value()
                    );
                }
            }
            println!();
        }
    }
    Ok(())
}
