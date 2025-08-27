use {
    anyhow::{anyhow, Result},
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account::{
        get_associated_token_address,
        instruction::create_associated_token_account_idempotent,
    },
    std::env,
};

// PumpFun specific constants - using smaller pubkeys to avoid compilation issues
const PUMPFUN_PROGRAM: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111"); // Placeholder
const PUMPFUN_GLOBAL: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111"); // Placeholder
const PUMPFUN_MINT_AUTHORITY: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111"); // Placeholder
const METAPLEX_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

// Initial reserves and supply
const INITIAL_VIRTUAL_SOL_RESERVES: u64 = 29_998_065_120;
const INITIAL_VIRTUAL_TOKEN_RESERVES: u64 = 73_000_000_000_000;

#[derive(Debug)]
struct TokenCreationParams {
    deployer_keypair: Keypair,
    token_mint_keypair: Keypair,
    metadata_uri: String,
    dev_buy_amount: f64,
    token_name: String,
    token_symbol: String,
    token_description: String,
}

struct PumpFun;

impl PumpFun {
    // Creates token creation + buy instructions
    pub fn get_create_buy_instruction(
        params: &TokenCreationParams,
    ) -> Result<Vec<Instruction>> {
        println!("Building token creation + dev buy instructions...");
        
        let TokenCreationParams {
            deployer_keypair,
            token_mint_keypair,
            metadata_uri,
            dev_buy_amount,
            token_name,
            token_symbol,
            token_description: _,
        } = params;

        let mint_pubkey = token_mint_keypair.pubkey();
        let deployer_pubkey = deployer_keypair.pubkey();

        println!("Mint Address: {}", mint_pubkey);
        println!("Deployer Address: {}", deployer_pubkey);
        println!("Dev Buy Amount: {} SOL", dev_buy_amount);

        // Validate inputs
        if token_name.is_empty() || token_symbol.is_empty() || metadata_uri.is_empty() {
            return Err(anyhow!("Token name, symbol, and metadata URI cannot be empty"));
        }

        if *dev_buy_amount <= 0.0 {
            return Err(anyhow!("Dev buy amount must be positive"));
        }

        println!("Calculating PDAs...");
        let bonding_curve = get_pda(&mint_pubkey, &PUMPFUN_PROGRAM)?;
        let (metadata_account, _) = Pubkey::find_program_address(
            &[b"metadata", METAPLEX_PROGRAM_ID.as_ref(), mint_pubkey.as_ref()],
            &METAPLEX_PROGRAM_ID,
        );

        let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint_pubkey);
        let deployer_token_account = get_associated_token_address(&deployer_pubkey, &mint_pubkey);

        println!("Bonding Curve: {}", bonding_curve);
        println!("Associated Bonding Curve: {}", associated_bonding_curve);
        println!("Metadata Account: {}", metadata_account);
        println!("Deployer Token Account: {}", deployer_token_account);

        // Build token data
        println!("Building creation instruction data...");
        let token_data = Self::build_create_instruction_data(
            token_name,
            token_symbol,
            metadata_uri,
            &deployer_pubkey,
        )?;
        println!("Creation instruction data built ({} bytes)", token_data.len());

        // Create the token creation instruction
        println!("Creating token creation instruction...");
        let create_instruction = Instruction::new_with_bytes(
            PUMPFUN_PROGRAM,
            &token_data,
            Self::get_create_instruction_accounts(
                mint_pubkey,
                bonding_curve,
                associated_bonding_curve,
                metadata_account,
                deployer_pubkey,
            ),
        );

        // Create ATA instruction
        println!("Creating ATA instruction...");
        let ata_instruction = create_associated_token_account_idempotent(
            &deployer_pubkey,
            &deployer_pubkey,
            &mint_pubkey,
            &spl_token::id(),
        );

        // Create dev buy instruction
        println!("Creating dev buy instruction...");
        let buy_instruction = Self::create_dev_buy_instruction(
            dev_buy_amount,
            mint_pubkey,
            bonding_curve,
            associated_bonding_curve,
            deployer_token_account,
            deployer_pubkey,
        )?;

        println!("All instructions built successfully");
        println!("Total instructions: 3 (Create + ATA + Dev Buy)");

        Ok(vec![create_instruction, ata_instruction, buy_instruction])
    }

    // Gets accounts for create instruction
    fn get_create_instruction_accounts(
        mint_pubkey: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
        metadata_account: Pubkey,
        deployer_pubkey: Pubkey,
    ) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(mint_pubkey, true),
            AccountMeta::new_readonly(PUMPFUN_MINT_AUTHORITY, false),
            AccountMeta::new(bonding_curve, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new_readonly(PUMPFUN_GLOBAL, false),
            AccountMeta::new_readonly(METAPLEX_PROGRAM_ID, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new(deployer_pubkey, true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(PUMPFUN_MINT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMPFUN_PROGRAM, false),
        ]
    }

    // Creates dev buy instruction
    fn create_dev_buy_instruction(
        dev_buy_amount: &f64,
        mint_pubkey: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
        deployer_token_account: Pubkey,
        deployer_pubkey: Pubkey,
    ) -> Result<Instruction> {
        println!("Calculating dev buy amounts...");
        let buy_amount_lamports = (dev_buy_amount * 1_000_000_000.0) as u64;
        println!("Buy amount in lamports: {}", buy_amount_lamports);
        
        let (tokens_to_receive, max_sol_cost, _) = Self::get_amount_out(
            buy_amount_lamports,
            INITIAL_VIRTUAL_SOL_RESERVES,
            INITIAL_VIRTUAL_TOKEN_RESERVES,
        );
        println!("Tokens to receive: {}", tokens_to_receive);
        println!("Max SOL cost: {}", max_sol_cost);
        
        let tokens_with_slippage = (tokens_to_receive * 85) / 100; // 15% slippage
        println!("Tokens with slippage: {} (15% slippage)", tokens_with_slippage);

        println!("Building dev buy instruction data...");
        let buy_instruction_data = Self::build_buy_instruction_data(tokens_with_slippage, max_sol_cost);
        println!("Buy instruction data built ({} bytes)", buy_instruction_data.len());
        
        let creator_vault = get_creator_vault_pda(&deployer_pubkey)?;
        println!("Creator vault: {}", creator_vault);

        println!("Creating dev buy instruction...");
        let instruction = Instruction {
            program_id: PUMPFUN_PROGRAM,
            accounts: vec![
                AccountMeta::new_readonly(PUMPFUN_GLOBAL, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(associated_bonding_curve, false),
                AccountMeta::new(deployer_token_account, false),
                AccountMeta::new(deployer_pubkey, true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new(creator_vault, false),
            ],
            data: buy_instruction_data,
        };

        println!("Dev buy instruction created successfully");
        Ok(instruction)
    }

    // Builds token data for creation instruction  
    fn build_create_instruction_data(
        token_name: &str,
        token_symbol: &str,
        metadata_uri: &str,
        deployer_pubkey: &Pubkey,
    ) -> Result<Vec<u8>> {
        let capacity = 8 + 4 + token_name.len() + 4 + token_symbol.len() + 4 + metadata_uri.len() + 32;
        let mut token_data = Vec::with_capacity(capacity);
        
        // Add discriminator for "create" instruction
        token_data.extend_from_slice(&[0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x00, 0x00]); // "create" + padding
        
        // Add name with length validation
        if token_name.len() > u32::MAX as usize {
            return Err(anyhow!("Token name too long"));
        }
        token_data.extend_from_slice(&(token_name.len() as u32).to_le_bytes());
        token_data.extend_from_slice(token_name.as_bytes());
        
        // Add symbol with length validation
        if token_symbol.len() > u32::MAX as usize {
            return Err(anyhow!("Token symbol too long"));
        }
        token_data.extend_from_slice(&(token_symbol.len() as u32).to_le_bytes());
        token_data.extend_from_slice(token_symbol.as_bytes());
        
        // Add URI with length validation
        if metadata_uri.len() > u32::MAX as usize {
            return Err(anyhow!("Metadata URI too long"));
        }
        token_data.extend_from_slice(&(metadata_uri.len() as u32).to_le_bytes());
        token_data.extend_from_slice(metadata_uri.as_bytes());
        
        // Add creator pubkey
        token_data.extend_from_slice(&deployer_pubkey.to_bytes());
        
        Ok(token_data)
    }

    // Builds buy instruction data
    fn build_buy_instruction_data(tokens_with_slippage: u64, max_sol_cost: u64) -> Vec<u8> {
        let mut buy_instruction_data = vec![
            0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ];
        
        buy_instruction_data[8..16].copy_from_slice(&tokens_with_slippage.to_le_bytes());
        buy_instruction_data[16..24].copy_from_slice(&max_sol_cost.to_le_bytes());
        
        buy_instruction_data
    }

    // Calculates amount out for bonding curve  
    fn get_amount_out(
        amount_in: u64,
        virtual_sol_reserves: u64,
        virtual_token_reserves: u64,
    ) -> (u64, u64, u64) {
        if virtual_sol_reserves == 0 {
            return (0, amount_in, 0);
        }

        let tokens_out = (amount_in as u128)
            .checked_mul(virtual_token_reserves as u128)
            .and_then(|result| result.checked_div(virtual_sol_reserves as u128))
            .unwrap_or(0) as u64;
        
        (tokens_out, amount_in, 0)
    }
}

// Gets PDA for bonding curve  
fn get_pda(mint: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Simple Token Creator using get_create_buy_instruction");
    println!("=====================================================");

    // Load configuration from environment
    let token_name = env::var("TOKEN_NAME").unwrap_or_else(|_| "My Token".to_string());
    let token_symbol = env::var("TOKEN_SYMBOL").unwrap_or_else(|_| "MTK".to_string());
    let metadata_uri = env::var("TOKEN_URI").unwrap_or_else(|_| "https://example.com/metadata.json".to_string());
    let dev_buy_amount: f64 = env::var("INITIAL_SOL_AMOUNT")
        .unwrap_or_else(|_| "0.1".to_string())
        .parse()
        .unwrap_or(0.1);

    // Parse private key
    let private_key = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY environment variable not set");
    
    let keypair = if private_key.starts_with('[') {
        // Array format
        let bytes: Vec<u8> = serde_json::from_str(&private_key)
            .expect("Failed to parse private key array");
        Keypair::from_bytes(&bytes)
            .expect("Invalid private key bytes")
    } else {
        // Base58 format
        Keypair::from_base58_string(&private_key)
    };

    println!("🔑 Wallet loaded: {}", keypair.pubkey());
    println!("📋 Token Configuration:");
    println!("   Name: {}", token_name);
    println!("   Symbol: {}", token_symbol);
    println!("   Metadata URI: {}", metadata_uri);
    println!("   Initial Buy Amount: {} SOL", dev_buy_amount);
    println!();

    // Generate new mint keypair for the token
    let token_mint_keypair = Keypair::new();
    println!("🎯 Generated new mint address: {}", token_mint_keypair.pubkey());

    // Create token creation parameters
    let params = TokenCreationParams {
        deployer_keypair: keypair,
        token_mint_keypair,
        metadata_uri,
        dev_buy_amount,
        token_name,
        token_symbol,
        token_description: "Token created with PumpFun Creator".to_string(),
    };

    println!("\n🔄 Step 1: Building token creation + buy instructions...");
    
    // Use get_create_buy_instruction from PumpFun
    let instructions = PumpFun::get_create_buy_instruction(&params)
        .expect("Failed to build instructions");
    
    println!("✅ Instructions built successfully!");
    println!("   📝 Total instructions: {}", instructions.len());
    println!("   📋 Instructions:");
    println!("     1. Token Creation (PumpFun)");
    println!("     2. Associated Token Account Creation (SPL)");
    println!("     3. Initial Buy (PumpFun)");
    println!();

    // Get recent blockhash
    let rpc_url = env::var("RPC_ENDPOINT").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let rpc_client = solana_client::rpc_client::RpcClient::new(rpc_url);
    let recent_blockhash = rpc_client.get_latest_blockhash()
        .expect("Failed to get recent blockhash");

    // Create transaction
    let mut transaction = Transaction::new_with_payer(
        &instructions,
        Some(&params.deployer_keypair.pubkey()),
    );
    
    // Sign with both keypairs
    transaction.sign(&[&params.deployer_keypair, &params.token_mint_keypair], recent_blockhash);
    
    println!("🔄 Step 2: Sending transaction to Solana...");
    println!("   📝 Transaction size: {} bytes", transaction.message.serialize().len());
    
    // Send transaction
    let signature = rpc_client.send_and_confirm_transaction(&transaction)
        .expect("Transaction failed");

    println!("\n🎉 Token Creation and Initial Buy Complete!");
    println!("==========================================");
    println!("   🎯 Token Mint: {}", params.token_mint_keypair.pubkey());
    println!("   📝 Transaction: {}", signature);
    println!("   🔍 Explorer: https://solscan.io/tx/{}", signature);
    println!("   🪙 Token Explorer: https://solscan.io/token/{}", params.token_mint_keypair.pubkey());
    
    // Save mint address for future reference
    std::fs::write("created_token_mint.txt", params.token_mint_keypair.pubkey().to_string())
        .expect("Failed to save mint address");

    println!("\n💡 Next Steps:");
    println!("   1. Wait for transaction confirmation");
    println!("   2. Check your wallet for the new tokens");
    println!("   3. Use the mint address to trade or monitor the token");
    println!("   4. The mint address has been saved to 'created_token_mint.txt'");

    Ok(())
}
