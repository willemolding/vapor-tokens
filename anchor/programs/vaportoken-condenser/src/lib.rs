use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};
use gnark_verifier_solana::{proof::GnarkProof, verifier::GnarkVerifier, witness::GnarkWitness};
use vaportoken_transfer_hook::MerkleTreeAccount;

mod vk;

declare_id!("LfXPYkVoeNy5933hcHZChMHREpwaNvnxpT1v6xdxajG");

#[program]
pub mod condenser {
    use super::*;

    pub fn condense(
        ctx: Context<Condense>,
        proof_bytes: Vec<u8>,
        pub_witness_bytes: Vec<u8>,
    ) -> Result<()> {
        let proof = GnarkProof::from_bytes(&proof_bytes).unwrap();
        let pub_witness = GnarkWitness::from_bytes(&pub_witness_bytes).unwrap();

        // Deserialize public inputs
        let recipient = pub_witness.entries[0..32]
            .iter()
            .map(|&b| b[0]) // check this
            .collect::<Vec<u8>>();
        let amount = pub_witness.entries[32];
        let merkle_root = pub_witness.entries[33];

        // Verify proof
        const NR_INPUTS: usize = vk::VK.nr_pubinputs;
        let mut verifier: GnarkVerifier<NR_INPUTS> = GnarkVerifier::new(&vk::VK);
        verifier
            .verify(proof, pub_witness)
            .map_err(|_| ErrorCode::InvalidProof)?;

        // check the root is a known root in our tree

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

        mint_to(cpi_ctx, 1)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Condense<'info> {
    /// The Token-2022 mint
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
    pub tree_account: AccountLoader<'info, MerkleTreeAccount>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("bad amount")]
    BadAmount,
    #[msg("invalid proof")]
    InvalidProof,
}
