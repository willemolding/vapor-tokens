use base64::{Engine, engine::general_purpose::STANDARD as Base64};
use borsh::BorshDeserialize;
use solana_client::{
    rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient},
    rpc_config::{CommitmentConfig, RpcTransactionConfig},
    rpc_response::OptionSerializer,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::{str::FromStr, time::Duration};

use crate::{TRANSFERS, TransferEvent};

pub fn sync(db: &redb::Database, rpc_url: &str, mint: &str) -> anyhow::Result<()> {
    let client = RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(30),
        CommitmentConfig::confirmed(),
    );

    // Anchor on the mint
    let mint = Pubkey::from_str(mint)?;

    let mut before: Option<Signature> = None;

    tracing::info!("Starting sync from mint {}", mint);
    loop {
        // Fetch one page (newest -> oldest)
        let page = client.get_signatures_for_address_with_config(
            &mint,
            GetConfirmedSignaturesForAddress2Config {
                limit: Some(1000),
                before: before,
                until: None,
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )?;

        if page.is_empty() {
            break;
        }

        for info in &page {
            // Skip failed txns right away
            if info.err.is_some() {
                continue;
            }

            let sig = Signature::from_str(&info.signature)?;

            let tx = client.get_transaction_with_config(
                &sig,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )?;

            if let Some(meta) = tx.transaction.meta {
                if let OptionSerializer::Some(logs) = meta.log_messages {
                    // Did our hook program run?
                    let ran_hook = logs
                        .iter()
                        .any(|l| l.contains(vaportoken_transfer_hook::ID.to_string().as_str()));

                    if ran_hook {
                        for line in logs {
                            if let Some(data) = line.strip_prefix("Program data: ") {
                                tracing::debug!("Slot {}: Found transfer event", tx.slot);

                                let bytes = Base64.decode(data)?;
                                let event: TransferEvent =
                                    TransferEvent::try_from_slice(&bytes[8..])?;
                                tracing::debug!("  Event data: {:?}", event);

                                let write_txn = db.begin_write()?;
                                {
                                    let mut table = write_txn.open_table(TRANSFERS)?;

                                    if table.insert(&tx.slot, &event)?.is_some() {
                                        tracing::debug!(
                                            "Existing spend found. Assuming scan complete"
                                        );
                                        return Ok(());
                                    }
                                }
                                write_txn.commit()?;
                            }
                        }
                    }
                }
            }
        }

        // Move cursor older: set `before` to the *oldest* signature in this page.
        before = Some(Signature::from_str(&page.last().unwrap().signature)?);

        // Reached the end (or at least: no more pages)
        if page.len() < 1000 {
            break;
        }
    }

    Ok(())
}
