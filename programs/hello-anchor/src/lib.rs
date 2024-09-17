use anchor_lang::prelude::*;
use anchor_spl::token::Transfer as SplTransfer;
use anchor_spl::token::{self, Token, TokenAccount};
use solana_program::system_instruction;

// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("8ZVMFHqxtWmRP9HTUjWfbtbrQszMrXwq7Z5cejrnEmMF");

#[program]
mod hello_anchor {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let bids = &mut ctx.accounts.bids;
        let user = &mut ctx.accounts.user;

        bids.total_bids = 0;
        bids.bump = ctx.bumps.bids;
        bids.admin = *user.to_account_info().key;
        Ok(())
    }

    pub fn add_bid(ctx: Context<AddBid>, url: String, starting_bid: f64) -> Result<()> {
        let bids = &mut ctx.accounts.bids;
        let user = &mut ctx.accounts.user;

        if user.key() != bids.admin {
            return Err(ProgramError::IllegalOwner.into());
        }

        let item = ItemStruct {
            id: bids.total_bids.checked_add(1).unwrap(),
            url: url.to_string(),
            starting_bid: starting_bid,
            owner_address: *user.to_account_info().key,
            bidding_open: true,
            details_list: Vec::new(),
        };

        bids.bids_list.push(item);

        bids.total_bids = bids.total_bids.checked_add(1).unwrap();
        Ok(())
    }

    pub fn place_bid(ctx: Context<PlaceBid>, find_id: u64, my_bid: f64) -> Result<()> {
        let bids = &mut ctx.accounts.bids;
        let destination = &ctx.accounts.to_ata;
        let source = &ctx.accounts.from_ata;
        let authority = &ctx.accounts.user;
        let token_program = &ctx.accounts.token_program;

        // Create the transfer instruction
        let transfer_instruction = system_instruction::transfer(
            ctx.accounts.user.key,
            ctx.accounts.to.key,
            (0.02 * 1_000_000_000.0) as u64,
        );

        // Invoke the transfer instruction
        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.to.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        let cpi_accounts = SplTransfer {
            from: source.to_account_info().clone(),
            to: destination.to_account_info().clone(),
            authority: authority.to_account_info().clone(),
        };
        let cpi_program = token_program.to_account_info();

        token::transfer(
            CpiContext::new(cpi_program, cpi_accounts),
            (my_bid * 1_000_000_000.0) as u64,
        )?;

        let new_bid = BidItemStruct {
            bidder_address: *ctx.accounts.user.key,
            bid_amount: my_bid,
        };

        for item in bids.bids_list.iter_mut() {
            if item.id == find_id {
                // Ensure the bidding is still open
                require!(item.bidding_open, CustomError::BiddingAlreadyClosed);

                // Ensure the placing bid amount is equal to or greater than the starting bid
                require!(my_bid >= item.starting_bid, CustomError::BidTooLow);

                item.details_list.push(new_bid);
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn end_bid(ctx: Context<EndBid>, find_id: u64) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let bids = &mut ctx.accounts.bids;

        if user.key() != bids.admin {
            return Err(ProgramError::IllegalOwner.into());
        }

        let item = &mut bids.bids_list[(find_id as usize) - 1];

        // Ensure the bidding is still open
        require!(item.bidding_open, CustomError::BiddingAlreadyClosed);

        // Find the highest bid
        if let Some(highest_bid) = item
            .details_list
            .iter()
            .max_by(|a, b| a.bid_amount.partial_cmp(&b.bid_amount).unwrap())
        {
            item.owner_address = highest_bid.bidder_address;
        } else {
            return Err(CustomError::NoBids.into());
        }

        // Close the bidding
        item.bidding_open = false;

        Ok(())
    }

    pub fn return_tokens(ctx: Context<ReturnTokens>, amount: u64) -> Result<()> {
        let seeds = &[b"bid1".as_ref(), &[255]]; // Correctly access the bump seed
        let signer = &[&seeds[..]];

        // Create the transfer instruction
        let transfer_instruction =
            system_instruction::transfer(ctx.accounts.user.key, ctx.accounts.admin.key, amount);

        // Invoke the transfer instruction
        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        let cpi_accounts = SplTransfer {
            from: ctx.accounts.pda_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pda_account.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();

        token::transfer(
            CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
            amount * 4,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        seeds = [b"bid1"], // optional seeds for pda
        bump,             // bump seed for pda
        payer = user,
        space = 5000 // 10240 is max
    )]
    pub bids: Account<'info, AllBids>,

    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum CustomError {
    #[msg("Bidding is already closed.")]
    BiddingAlreadyClosed,
    #[msg("No bids found.")]
    NoBids,
    #[msg("Bid amount is too low.")]
    BidTooLow,
}

#[derive(Accounts)]
pub struct AddBid<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bid1"], // optional seeds for pda
        bump = bids.bump,  // bump seed for pda
    )]
    pub bids: Account<'info, AllBids>,
}

#[derive(Accounts)]
pub struct EndBid<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bid1"], // optional seeds for pda
        bump = bids.bump,  // bump seed for pda
    )]
    pub bids: Account<'info, AllBids>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bid1"], // optional seeds for pda
        bump = bids.bump,  // bump seed for pda
    )]
    pub bids: Account<'info, AllBids>,

    #[account(mut)]
    /// CHECK: Account info of the product owner where ownership transfers
    pub to: AccountInfo<'info>,

    #[account(mut)]
    pub from_ata: Account<'info, TokenAccount>, // associated token account.

    #[account(mut)]
    pub to_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReturnTokens<'info> {
    #[account(
        mut,
        seeds = [b"bid1"], // optional seeds for pda
        bump = bids.bump,  // bump seed for pda
    )]
    pub bids: Account<'info, AllBids>,

    #[account(mut)]
    pub pda_account: AccountInfo<'info>, // PDA account that holds tokens

    #[account(mut)]
    pub pda_token_account: Account<'info, TokenAccount>, // Token account of the PDA

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>, // Token account of the user

    pub token_program: Program<'info, Token>,

    #[account(mut)]
    /// CHECK: Account info of the product owner where ownership transfers
    pub admin: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct BidItemStruct {
    pub bidder_address: Pubkey,
    pub bid_amount: f64,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ItemStruct {
    pub id: u64,
    pub url: String,
    pub starting_bid: f64,
    pub owner_address: Pubkey,
    pub bidding_open: bool,
    pub details_list: Vec<BidItemStruct>,
}

#[account]
pub struct AllBids {
    pub total_bids: u64, // 8 bytes
    pub bump: u8,        // 1 byte
    pub bids_list: Vec<ItemStruct>,
    pub admin: Pubkey,
}

#[account]
pub struct NewAccount {
    data: u64,
}

// CREATE a to recover bid function.
// add some adjustments and its done.
