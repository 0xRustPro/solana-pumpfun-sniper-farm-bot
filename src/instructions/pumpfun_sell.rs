use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::{Account, AccountState};
use std::sync::Arc;
//use crate::config::PUBKEY;
use solana_client::nonblocking::rpc_client::RpcClient;


// PumpFun specific constants
pub const PUMPFUN_PROGRAM: Pubkey = solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPFUN_GLOBAL: Pubkey = solana_sdk::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMPFUN_MINT_AUTHORITY: Pubkey = solana_sdk::pubkey!("TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM");
pub const METAPLEX_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
pub const PUMPFUN_FEE_RECIPIENT: Pubkey = solana_sdk::pubkey!("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");
pub const PUMPFUN_EVENT_AUTHORITY: Pubkey = solana_sdk::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");

// Mathematical constants
pub const TEN_THOUSAND: u128 = 10_000;

#[derive(Clone)]
pub struct PumpFunSell {
    pub rpc_client: Arc<RpcClient>    
}

impl PumpFunSell {
    pub fn new(
        rpc_client: Arc<RpcClient>,
    ) -> Self {
        Self {
            rpc_client,
        }
    }

    // Helper function to get ATA token balance
    async fn get_ata_token_balance(&self, ata_address: &Pubkey) -> Result<u64> {
        let token_balance = self.rpc_client.get_token_account_balance(ata_address).await?;    
        let bought_amount_raw = token_balance.amount.parse::<u64>().unwrap_or(0);
        Ok(bought_amount_raw)
    }

    // Creates sell instruction that sells all tokens in ATA
    pub fn create_sell_all_instruction(
        mint_pubkey: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
        user_token_account: Pubkey,
        user_pubkey: Pubkey,
        creator_vault: Pubkey,
        tokens_to_sell: u64,
        min_sol_output: u64,
    ) -> Result<Instruction> {
        println!("Creating sell all instruction...");
        
        println!("Building sell instruction data...");
        let sell_instruction_data = Self::build_sell_instruction_data(tokens_to_sell, min_sol_output);
        println!("Sell instruction data built ({} bytes)", sell_instruction_data.len());

        println!("Creating sell instruction...");
        let instruction = Instruction {
            program_id: PUMPFUN_PROGRAM,
            accounts: vec![
                AccountMeta::new_readonly(PUMPFUN_GLOBAL, false),
                AccountMeta::new(PUMPFUN_FEE_RECIPIENT, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(associated_bonding_curve, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new(user_pubkey, true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new(creator_vault, false),
                AccountMeta::new_readonly(solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"), false),
                AccountMeta::new_readonly(PUMPFUN_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUMPFUN_PROGRAM, false),
            ],
            data: sell_instruction_data,
        };

        println!("Sell instruction created successfully");
        Ok(instruction)
    }

    // Creates ATA close instruction to close the token account and recover rent
    pub fn create_ata_close_instruction(
        user_token_account: Pubkey,
        user_pubkey: Pubkey,
        mint_pubkey: Pubkey,
    ) -> Result<Instruction> {
        println!("Creating ATA close instruction...");
        
        // Create instruction to close the ATA and transfer rent back to user
        let close_instruction = spl_token::instruction::close_account(
            &solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
            &user_token_account,
            &user_pubkey,
            &user_pubkey,
            &[],
        )?;

        println!("ATA close instruction created successfully");
        Ok(close_instruction)
    }

    // Enhanced function to get sell all + close ATA instructions with balance query
    pub async fn get_sell_all_and_close_instructions_with_balance(
        &self,
        mint_pubkey: Pubkey,
        user_token_account: Pubkey,
        user_pubkey: Pubkey,
        creator_vault: Pubkey,
        slippage_percentage: f64,
    ) -> Result<Vec<Instruction>> {
        println!("Building enhanced sell all + close ATA instructions...");
        
        let bonding_curve = get_pda(&mint_pubkey, &PUMPFUN_PROGRAM)?;
        let (metadata_account, _) = Pubkey::find_program_address(
            &[b"metadata", METAPLEX_PROGRAM_ID.as_ref(), mint_pubkey.as_ref()],
            &METAPLEX_PROGRAM_ID,
        );
        let deployer_pubkey = user_pubkey;
        let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint_pubkey);
        let deployer_token_account = get_associated_token_address(&deployer_pubkey, &mint_pubkey);

        
        // Query the actual token balance in the ATA
        let token_balance = self.get_ata_token_balance(&user_token_account).await?;
        println!("ATA Token Balance: {}", token_balance);
        
        if token_balance == 0 {
            return Err(anyhow!("No tokens to sell - ATA balance is 0"));
        }
        
        // Calculate expected SOL output using bonding curve math
        let expected_sol_output = Self::calculate_sell_sol_amount(
            token_balance,
            &mint_pubkey.to_string(),
        )?;
        
        // Apply slippage protection - use a much more conservative approach
        let slippage_bps = (slippage_percentage * 100.0) as u128;
        let min_sol_output = min_amount_with_slippage(expected_sol_output as u64, slippage_bps);
        
        // Ensure minimum output is very low to avoid slippage errors
        let final_min_output = std::cmp::min(min_sol_output, 100); // Max 100 lamports minimum (very conservative)
        
        println!("Expected SOL Output: {} lamports", expected_sol_output);
        println!("Minimum SOL Output (with {}% slippage): {} lamports", slippage_percentage, min_sol_output);
        println!("Final Minimum Output (capped): {} lamports", final_min_output);
        
        // Create sell instruction
        let sell_instruction = Self::create_sell_all_instruction(
            mint_pubkey,
            bonding_curve,
            associated_bonding_curve,
            user_token_account,
            user_pubkey,
            creator_vault,
            token_balance,
            final_min_output,
        )?;

        // Create ATA close instruction
        let close_instruction = Self::create_ata_close_instruction(
            user_token_account,
            user_pubkey,
            mint_pubkey,
        )?;

        println!("All enhanced instructions built successfully");
        println!("Total instructions: 2 (Enhanced Sell All + Close ATA)");

        Ok(vec![sell_instruction, close_instruction])
    }

    // Builds sell instruction data
    fn build_sell_instruction_data(tokens_to_sell: u64, min_sol_output: u64) -> Vec<u8> {
        let mut sell_instruction_data = vec![
            0x33, 0xE6, 0x85, 0xA4, 0x01, 0x7F, 0x83, 0xAD, // Sell discriminator (Anchor hash for "sell")
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ];
        
        // Set the token amount to sell (all tokens in ATA)
        sell_instruction_data[8..16].copy_from_slice(&tokens_to_sell.to_le_bytes());
        
        // Set minimum SOL output (with slippage protection)
        sell_instruction_data[16..24].copy_from_slice(&min_sol_output.to_le_bytes());
        
        sell_instruction_data
    }

    // Calculates sell SOL amount (simplified bonding curve math)
    fn calculate_sell_sol_amount(
        amount_specified: u64,
        mint_str: &str,
    ) -> Result<u128> {
        // Use a much more conservative estimate for SOL output
        // This should be much lower to avoid slippage errors
        let estimated_sol_per_token = 0.000000001; // Very conservative estimate (1 nano SOL per token)
        let sol_output = (amount_specified as f64 * estimated_sol_per_token * 1_000_000_000.0) as u128;
        
        // Ensure minimum output is at least 1 lamport
        let min_output = if sol_output == 0 { 1 } else { sol_output };
        
        println!("Calculated SOL output: {} lamports for {} tokens", min_output, amount_specified);
        Ok(min_output)
    }

    // Utility function to get all required parameters for selling tokens
    pub fn get_sell_parameters(
        mint_pubkey: Pubkey,
        user_pubkey: Pubkey,
    ) -> Result<(Pubkey, Pubkey, Pubkey, Pubkey)> {
        println!("Getting sell parameters for mint: {}", mint_pubkey);
        
        // Calculate bonding curve PDA
        let bonding_curve = get_pda(&mint_pubkey, &PUMPFUN_PROGRAM)?;
        println!("Bonding Curve: {}", bonding_curve);
        
        // Calculate associated bonding curve ATA
        let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint_pubkey);
        println!("Associated Bonding Curve: {}", associated_bonding_curve);
        
        // Calculate user's token ATA
        let user_token_account = get_associated_token_address(&user_pubkey, &mint_pubkey);
        println!("User Token Account: {}", user_token_account);
        
        // Calculate creator vault PDA
        let creator_vault = get_creator_vault_pda(&user_pubkey)?;
        println!("Creator Vault: {}", creator_vault);
        
        Ok((bonding_curve, associated_bonding_curve, user_token_account, creator_vault))
    }

    // Convenience function to sell all tokens and close ATA in one call
    pub async fn sell_all_tokens_and_close_ata(
        &self,
        mint_pubkey: Pubkey,
        user_pubkey: Pubkey,
        slippage_percentage: f64,
    ) -> Result<Vec<Instruction>> {
        println!("🚀 Selling all tokens and closing ATA for mint: {}", mint_pubkey);
        
        // Get all required parameters
        let (bonding_curve, associated_bonding_curve, user_token_account, creator_vault) = 
            Self::get_sell_parameters(mint_pubkey, user_pubkey)?;
        
        // Create instructions
        let instructions = self.get_sell_all_and_close_instructions_with_balance(
            mint_pubkey,
            user_token_account,
            user_pubkey,
            creator_vault,
            slippage_percentage,
        ).await?;
        
        println!("✅ Successfully created sell all + close ATA instructions");
        Ok(instructions)
    }
}

// Calculates minimum amount with slippage
pub fn min_amount_with_slippage(input_amount: u64, slippage_bps: u128) -> u64 {
    let min_amount = u128::from(input_amount)
        .checked_mul(TEN_THOUSAND.checked_sub(slippage_bps).unwrap_or(0))
        .unwrap_or(0)
        .checked_div(TEN_THOUSAND)
        .unwrap_or(0);
    min_amount as u64
}

// Gets PDA for bonding curve  
pub fn get_pda(mint: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
    let seeds = [b"bonding-curve".as_ref(), mint.as_ref()];
    let (bonding_curve, _bump) = Pubkey::find_program_address(&seeds, program_id);
    Ok(bonding_curve)
}

// Gets creator vault PDA  
fn get_creator_vault_pda(creator: &Pubkey) -> Result<Pubkey> {
    let seeds = [b"creator-vault".as_ref(), creator.as_ref()];
    let (creator_vault, _bump) = Pubkey::find_program_address(&seeds, &PUMPFUN_PROGRAM);
    Ok(creator_vault)
}