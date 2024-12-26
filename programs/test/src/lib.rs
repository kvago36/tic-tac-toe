use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_lang::system_program::{transfer, Transfer};
use std::fmt;

declare_id!("8G1UnHQNpbc71P4MT8dtCw7CnFs6MxFSBxzmjyJkebMx");

#[program]
mod test {
    use super::*;

    pub const DURATION: i64 = 600;

    pub fn init_game(ctx: Context<InitGame>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        ctx.accounts.game.amount = amount;
        ctx.accounts.game.owner = ctx.accounts.user.key();
        ctx.accounts.game.state = GameState::Awaiting;
        ctx.accounts.game.created_at = current_timestamp;
        ctx.accounts.game.end_time = current_timestamp + DURATION;

        vault.owner = *ctx.accounts.user.key;
        vault.bump = ctx.bumps.vault;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );

        transfer(cpi_context, amount)?;

        Ok(())
    }

    pub fn join_game(ctx: Context<JoinGame>) -> Result<()> {
        let game = &mut ctx.accounts.game;

        require_keys_neq!(*ctx.accounts.user.key, ctx.accounts.vault.owner, MyError::AlreadyInGame);

        require_gte!(**ctx.accounts.user.to_account_info().lamports.borrow(), game.amount, MyError::InsufficientFunds);

        require_eq!(&game.state, &GameState::Awaiting, MyError::InvalidGameState);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );

        transfer(cpi_context, game.amount)?;

        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        game.state = GameState::Active;
        game.end_time = current_timestamp + DURATION;

        Ok(())
    }

    pub fn claim_back(_ctx: Context<ClaimBack>) -> Result<()> {
        // let game = &mut ctx.accounts.game;

        // let clock = Clock::get()?;
        // let current_timestamp = clock.unix_timestamp;

        // require_eq!(&game.state, &GameState::Awaiting, MyError::GameStillRunnig);

        // require_gte!(current_timestamp, game.created_at + DURATION, MyError::GameStillRunnig);

        Ok(())
    }

    pub fn test(ctx: Context<ClaimBack>) -> Result<()> {
        let amount_of_lamports = ctx.accounts.vault.to_account_info().lamports();

        require_keys_eq!(ctx.accounts.user.key(), ctx.accounts.vault.owner.key(), MyError::NotTheOwner);

        // msg!("user {}", ctx.accounts.user.key());
        // msg!("vault owner {}", ctx.accounts.vault.owner.key());

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= amount_of_lamports;
        **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += amount_of_lamports;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitGame<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
    init, 
    payer = user, 
    space = 8 + 40, // Space for the account
    seeds = [b"vault", user.key().as_ref()], 
    bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(init, payer = user, space = 8 + Game::INIT_SPACE)]
    pub game: Account<'info, Game>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", vault.owner.key().as_ref()], 
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub game: Account<'info, Game>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimBack<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump,
        constraint = game.state == GameState::Awaiting @ MyError::GameStillRunnig,
        constraint = clock.unix_timestamp > game.created_at @ MyError::GameNotReadyToClose,
        close = user,
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub game: Account<'info, Game>,

    pub system_program: Program<'info, System>,

    #[account(address = sysvar::clock::ID)]
    pub clock: Sysvar<'info, Clock>,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Awaiting,
    Active,
    Tie,
    Won { winner: Pubkey },
}

#[account]
pub struct Vault {
    pub owner: Pubkey, // The owner of the vault
    pub bump: u8,      // Bump seed for PDA
}

#[account]
#[derive(InitSpace)]
pub struct Game {
    owner: Pubkey,
    state: GameState,
    created_at: i64,
    end_time: i64,
    amount: u64,
    turn: u8,
}


impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameState::Awaiting => write!(f, "Waiting"),
            GameState::Active => write!(f, "In Progress"),
            GameState::Tie => write!(f, "Completed"),
            GameState::Won { winner } => write!(f, "Won by {}", winner),
        }
    }
}

#[error_code]
pub enum MyError {
    #[msg("You arent owner of the vault!")]
    NotTheOwner,
    #[msg("Can join only awaiting game!")]
    InvalidGameState,
    #[msg("Players can join game more than one time!")]
    AlreadyInGame,
    #[msg("Insufficient funds for this operation.")]
    InsufficientFunds,
    #[msg("Cant close runnig game!")]
    GameStillRunnig,
    #[msg("Cant close game yet!")]
    GameNotReadyToClose,    
}