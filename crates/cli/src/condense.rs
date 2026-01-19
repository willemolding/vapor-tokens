use std::str::FromStr;

use anchor_lang::InstructionData;
use ark_bn254::Fr as NoirField;
use borsh::BorshDeserialize;
use condenser_witness::CondenserWitness;
use light_bounded_vec::BoundedVec;
use redb::{ReadableDatabase, ReadableTable};
use solana_client::rpc_client::RpcClient;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_program::system_program;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use transfer_tree::TransferTreeExt;

use crate::{TRANSFERS, VAP_ADDR, prove::prove};

pub fn condense<const HEIGHT: usize>(
    db: &redb::Database,
    rpc_url: &str,
    payer: Keypair,
    mint: Pubkey,
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

    let (proof, proof_indices, root) = build_merkle_proof::<HEIGHT>(db, deposits[selection].0)?;
    let (proof, witness) = build_witness_and_prove(
        vapor_addr,
        deposit.1.amount,
        bs58::encode(addr_record.recipient).into_string().as_str(),
        &addr_record.secret,
        proof,
        proof_indices,
        root,
    )?;

    submit_proof(
        rpc_url,
        payer,
        mint,
        Pubkey::try_from_slice(&addr_record.recipient)?,
        proof,
        witness,
    )?;

    Ok(())
}

fn submit_proof(
    rpc_url: &str,
    payer: Keypair,
    mint: Pubkey,
    recipient: Pubkey,
    proof_bytes: Vec<u8>,
    pub_witness_bytes: Vec<u8>,
) -> anyhow::Result<()> {
    let client = RpcClient::new(rpc_url.to_string());

    let token_program = Pubkey::new_from_array(spl_token_2022::ID.to_bytes());
    let condenser_program = Pubkey::new_from_array(vaportoken_condenser::ID.to_bytes());
    let transfer_hook_program = Pubkey::new_from_array(vaportoken_transfer_hook::ID.to_bytes());

    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"mint_authority", mint.as_ref()], &condenser_program);
    let (tree_account, _) =
        Pubkey::find_program_address(&[b"merkle_tree", mint.as_ref()], &transfer_hook_program);
    let (withdrawn, _) = Pubkey::find_program_address(
        &[b"withdrawn", mint.as_ref(), recipient.as_ref()],
        &condenser_program,
    );

    let data = vaportoken_condenser::instruction::Condense {
        recipient: recipient.to_bytes().into(),
        proof_bytes,
        pub_witness_bytes,
    }
    .data();

    let recipient_ata = get_associated_token_address_with_program_id(
        &recipient.to_bytes().into(),
        &mint.to_bytes().into(),
        &spl_token_2022::ID,
    );

    let accounts = vec![
        AccountMeta::new(mint, false),
        AccountMeta::new(recipient_ata.to_bytes().into(), false),
        AccountMeta::new_readonly(mint_authority, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(tree_account, false),
        AccountMeta::new(withdrawn, false),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(system_program::ID.to_bytes().into(), false),
    ];

    let instruction = Instruction {
        program_id: condenser_program,
        accounts,
        data,
    };

    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
            instruction,
        ],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );
    let sig = client.send_and_confirm_transaction(&tx)?;

    println!(
        "View transaction https://solscan.io/tx/{}?cluster=devnet",
        sig
    );

    Ok(())
}

/// Build the Merkle proof for the transfer at the given index
/// using transfers from the db
fn build_merkle_proof<const HEIGHT: usize>(
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

fn build_witness_and_prove<const HEIGHT: usize>(
    vapor_addr: [u8; 32],
    amount: u64,
    recipient: &str,
    secret: &str,
    proof: [[u8; 32]; HEIGHT],
    proof_indices: [u8; HEIGHT],
    root: [u8; 32],
) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
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

    prove::<HEIGHT>(witness.clone())
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
