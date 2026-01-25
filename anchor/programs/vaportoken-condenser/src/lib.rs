use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};
use gnark_verifier_solana::{proof::GnarkProof, verifier::GnarkVerifier, witness::GnarkWitness};
use utils::unpack_bytes_from_le_fields;
use vaportoken_transfer_hook::MerkleTreeAccount;

mod vk;

declare_id!("Bs5oDuMEnM4VzseKjNndM4wgzZUhrWNJ2DRpiMp9xVFv");

#[program]
pub mod condenser {
    use vaportoken_transfer_hook::MerkleTree;

    use super::*;

    pub fn condense(
        ctx: Context<Condense>,
        recipient: Pubkey,
        proof_bytes: Vec<u8>,
        pub_witness_bytes: Vec<u8>,
    ) -> Result<()> {
        const NR_INPUTS: usize = vk::VK.nr_pubinputs;
        let proof = GnarkProof::from_bytes(&proof_bytes).unwrap();
        let pub_witness = GnarkWitness::<NR_INPUTS>::from_bytes(&pub_witness_bytes).unwrap();

        // Deserialize public inputs
        let recipient_bytes = unpack_bytes_from_le_fields(&pub_witness.entries[0..2], 32);
        let recipient_from_witness = Pubkey::new_from_array(
            recipient_bytes
                .as_slice()
                .try_into()
                .map_err(|_| ErrorCode::InvalidRecipient)?,
        );
        let amount = u64::from_be_bytes(pub_witness.entries[2][24..].try_into().unwrap());
        let merkle_root = pub_witness.entries[3];

        // Check recipient matches
        if recipient != recipient_from_witness {
            return Err(ErrorCode::RecipientMismatch.into());
        }

        // Verify proof
        let mut verifier: GnarkVerifier<NR_INPUTS> = GnarkVerifier::new(&vk::VK);
        verifier
            .verify(proof, pub_witness)
            .map_err(|_| ErrorCode::InvalidProof)?;

        // check the root is a known root in our tree
        let tree_account = ctx.accounts.tree_account.load()?;

        if !MerkleTree::is_known_root(&tree_account, merkle_root) {
            return Err(ErrorCode::MerkleRootNotInHistory.into());
        }

        // If passed then call mint_to CPI to mint new tokens
        let bump = ctx.bumps.mint_authority;
        let key = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint_authority", key.as_ref(), &[bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
            signer_seeds,
        );

        // Mint the delta between the total withdrawn and the amount specified in the proof
        let withdrawn = &mut ctx.accounts.withdrawn;
        let delta = amount
            .checked_sub(withdrawn.total_withdrawn)
            .ok_or(ErrorCode::BadAmount)?;

        // Store the new amount withdrawn
        withdrawn.total_withdrawn = amount;

        mint_to(cpi_ctx, delta)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(recipient: Pubkey)]
pub struct Condense<'info> {
    /// The Token-2022 mint
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Recipient token account (Token-2022)
    #[account(mut)]
    pub to: InterfaceAccount<'info, TokenAccount>,

    /// PDA that is set as the mint's mint_authority
    /// seeds must match whatever you used when setting authority
    #[account(
        seeds = [b"mint_authority", mint.key().as_ref()],
        bump
    )]
    /// CHECK: PDA authority only used for signing
    pub mint_authority: UncheckedAccount<'info>,

    /// Token-2022 program (TokenInterface works for both token programs)
    pub token_program: Interface<'info, TokenInterface>,

    /// The tree account that is updated with every transfer of the token
    #[account(
        seeds = [b"merkle_tree", mint.key().as_ref()],
        seeds::program = vaportoken_transfer_hook::ID,
        bump = tree_account.load()?.bump
    )]
    pub tree_account: AccountLoader<'info, MerkleTreeAccount>,

    /// Tracks the total withdrawn amount for each recipient
    /// This is how double-spends are prevented without nullifiers
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + 64,
        seeds = [b"withdrawn", mint.key().as_ref(), recipient.as_ref()],
        bump
    )]
    pub withdrawn: Account<'info, WithdrawnTracker>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct WithdrawnTracker {
    total_withdrawn: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("bad amount")]
    BadAmount,
    #[msg("invalid proof")]
    InvalidProof,
    #[msg("invalid recipient bytes")]
    InvalidRecipient,
    #[msg("recipient does not match public witness")]
    RecipientMismatch,
    #[msg("Merkle root not found in recent root history")]
    MerkleRootNotInHistory,
    #[msg("arithmetic overflow")]
    ArithmeticOverflow,
}
