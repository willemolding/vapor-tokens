use light_bounded_vec::BoundedVec;
use redb::{ReadableDatabase, ReadableTable};
use transfer_tree::TransferTreeExt;

use crate::TRANSFERS;

/// Build the Merkle proof for the transfer at the given index
/// using transfers from the db
pub(crate) fn build_merkle_proof<const HEIGHT: usize>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransferEvent;

    const HEIGHT: usize = 26;

    #[test]
    fn test_build_merkle_proof() {
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

        let (proof, _indices, _root) = build_merkle_proof::<HEIGHT>(&db, 5).unwrap();
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

        let (proof, _indices, root) = build_merkle_proof::<HEIGHT>(&db, 0).unwrap();
        assert_eq!(proof.len(), HEIGHT);
        println!("Root: {:?}", hex::encode(root));
    }
}
