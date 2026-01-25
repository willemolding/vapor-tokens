use std::str::FromStr;

use anchor_lang::InstructionData;
use ark_bn254::Fr as NoirField;
use borsh::BorshDeserialize;
use condenser_witness::CondenserWitness;
use redb::{ReadableDatabase, ReadableTable};
use solana_client::rpc_client::RpcClient;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_sdk_ids::system_program;
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};

use crate::build_merkle_proof::build_merkle_proof;
use crate::{TRANSFERS, VAP_ADDR, prove::prove};

/// Construct and submit the proof required to condense (mint) a number of vaporized tokens into their destination account
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
        &spl_token_2022::ID.to_bytes().into(),
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

    let create_ata_ix = create_associated_token_account_idempotent(
        &payer.pubkey().to_bytes().into(),
        &recipient.to_bytes().into(),
        &mint.to_bytes().into(),
        &spl_token_2022::ID.to_bytes().into(),
    );

    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
            create_ata_ix,
            instruction,
        ],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    println!("Submitting condense transaction...");
    let sig = client.send_and_confirm_transaction(&tx)?;
    println!(
        "Transaction accepted https://solscan.io/tx/{}?cluster=devnet",
        sig
    );

    Ok(())
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
