use anchor_lang::prelude::*;

declare_id!("AHRry3MwdUsLbNgfzx5bbpmzz52oRL7UgLgvuiWpV97w");

#[program]
pub mod vapor_tokens {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
