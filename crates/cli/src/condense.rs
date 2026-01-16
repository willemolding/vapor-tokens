use std::str::FromStr;

use ark_bn254::Fr as NoirField;
use condenser_witness::CondenserWitness;
use light_bounded_vec::BoundedVec;
use redb::{ReadableDatabase, ReadableTable};
use transfer_tree::TransferTreeExt;

use crate::{TRANSFERS, VAP_ADDR};

pub fn condense<const HEIGHT: usize>(
    db: &redb::Database,
    rpc_url: &str,
    mint: &str,
    vapor_addr: &str,
) -> anyhow::Result<()> {
    let vapor_addr: [u8; 32] = bs58::decode(vapor_addr)
        .into_vec()?
        .try_into()
        .expect("vapor_addr must be 32 bytes");

    let read_txn = db.begin_read()?;
    let (addr_record, deposits) = {
        let addresses = read_txn.open_table(VAP_ADDR)?;
        let transfers = read_txn.open_table(TRANSFERS)?;

        let addr_record = addresses
            .get(vapor_addr)?
            .ok_or_else(|| anyhow::anyhow!("Address not found"))?
            .value();

        let deposits = transfers
            .iter()?
            .enumerate()
            .filter_map(|(i, transfer)| {
                let (_, transfer) = transfer.ok()?;
                if transfer.value().to == vapor_addr {
                    Some((i, transfer.value()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        (addr_record, deposits)
    };

    println!("Which deposit would you like to condense?");
    for (i, deposit) in deposits.iter().enumerate() {
        println!("  [{}] Deposit {}", i, deposit.1.amount);
    }
    let mut selection = String::new();
    std::io::stdin().read_line(&mut selection)?;
    let selection: usize = selection.trim().parse()?;

    let deposit = &deposits[selection];

    let (proof, proof_indices, root) = build_proof::<HEIGHT>(db, deposits[selection].0)?;
    build_witness(
        vapor_addr,
        deposit.1.amount,
        bs58::encode(addr_record.recipient).into_string().as_str(),
        &addr_record.secret,
        proof,
        proof_indices,
        root,
    )?;

    Ok(())
}

/// Build the Merkle proof for the transfer at the given index
/// using transfers from the db
fn build_proof<const HEIGHT: usize>(
    db: &redb::Database,
    index: usize,
) -> anyhow::Result<([[u8; 32]; HEIGHT], [u8; HEIGHT], [u8; 32])> {
    let mut tree = transfer_tree::TransferTree::<HEIGHT>::new_empty();
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    let mut transfer_at_idx = None;
    let mut changelog_index = 0;

    let read_txn = db.begin_read()?;
    {
        let transfers = read_txn.open_table(TRANSFERS)?;
        for (i, result) in transfers.iter()?.enumerate() {
            let (_, transfer) = result?;
            match i {
                _ if i < index => {
                    tree.append_transfer(transfer.value().to, transfer.value().amount)?;
                }
                _ if i == index => {
                    transfer_at_idx = Some(transfer.value());
                    (changelog_index, _) = tree.append_transfer_with_proof(
                        transfer.value().to,
                        transfer.value().amount,
                        &mut proof,
                    )?;
                }
                _ if i > index => {
                    tree.append_transfer(transfer.value().to, transfer.value().amount)?;
                    tree.update_proof_from_changelog(changelog_index, index, &mut proof)
                        .unwrap();
                    changelog_index = tree.changelog_index();
                }
                _ => unreachable!(),
            };
        }
    }

    // sanity check that the proof verifies
    if let Some(transfer_at_idx) = transfer_at_idx {
        tree.validate_transfer_proof(transfer_at_idx.to, transfer_at_idx.amount, index, &proof)?;
    } else {
        anyhow::bail!("No transfer found at index {}", index);
    }

    Ok((
        proof.to_array().unwrap(),
        tree.proof_indices(index),
        tree.root(),
    ))
}

fn build_witness<const HEIGHT: usize>(
    vapor_addr: [u8; 32],
    amount: u64,
    recipient: &str,
    secret: &str,
    proof: [[u8; 32]; HEIGHT],
    proof_indices: [u8; HEIGHT],
    root: [u8; 32],
) -> anyhow::Result<()> {
    let secret = NoirField::from_str(&secret).expect("vapor_addr must be 32 bytes");

    let recipient: [u8; 32] = bs58::decode(recipient)
        .into_vec()?
        .try_into()
        .expect("recipient must be 32 bytes");

    let witness = CondenserWitness::builder()
        .recipient(recipient)
        .amount(amount)
        .merkle_root(root)
        .merkle_proof(proof)
        .merkle_proof_indices(proof_indices)
        .vapor_addr(vapor_addr)
        .secret(secret)
        .build();

    println!("{}", witness.to_toml());
    println!();
    println!("Vapor address: {}", bs58::encode(vapor_addr).into_string());
    println!("Recipient: {}", bs58::encode(recipient).into_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransferEvent;

    const HEIGHT: usize = 26;

    #[test]
    fn test_build_proof() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        let write_txn = db.begin_write().unwrap();
        {
            let mut transfers = write_txn.open_table(TRANSFERS).unwrap();
            for i in 0..10u64 {
                let transfer = TransferEvent {
                    to: [i as u8; 32],
                    amount: i * 100,
                };
                transfers.insert(&i, &transfer).unwrap();
            }
        }
        write_txn.commit().unwrap();

        let (proof, _indices, _root) = build_proof::<HEIGHT>(&db, 5).unwrap();
        assert_eq!(proof.len(), HEIGHT);
    }

    #[test]
    fn test_match_anchor_test() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        let write_txn = db.begin_write().unwrap();
        {
            let mut transfers = write_txn.open_table(TRANSFERS).unwrap();
            let transfer = TransferEvent {
                to: bs58::decode("DKp1YW5zcJBR4ujZnbW6gJWFXSWerS6CMJogV4tcfgNh")
                    .into_vec()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                amount: 1000,
            };
            transfers.insert(&0, &transfer).unwrap();
        }
        write_txn.commit().unwrap();

        let (proof, _indices, root) = build_proof::<HEIGHT>(&db, 0).unwrap();
        assert_eq!(proof.len(), HEIGHT);
        println!("Root: {:?}", hex::encode(root));
    }
}
