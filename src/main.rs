use {
    async_trait::async_trait,
    borsh::BorshDeserialize,
    carbon_core::{
        deserialize::{ArrangeAccounts, CarbonDeserialize},
        error::CarbonResult,
        instruction::{
            DecodedInstruction, InstructionMetadata, InstructionProcessorInputType,
            NestedInstructions,
        },
        metrics::MetricsCollection,
        processor::Processor,
    },
    carbon_log_metrics::LogMetrics,
    carbon_pumpfun_decoder::{
        instructions::{buy::Buy, sell::Sell, trade_event::TradeEvent, PumpfunInstruction}, PumpfunDecoder, PROGRAM_ID as PUMPFUN_PROGRAM_ID
    },
    carbon_yellowstone_grpc_datasource::YellowstoneGrpcGeyserClient,
    pumpfun_monitor::{
        config::{
            init_jito, init_nozomi, init_zslot, BUY_SOL_AMOUNT, CONFIRM_SERVICE, JITO_CLIENT, NOZOMI_CLIENT, PRIORITY_FEE, PUBKEY, RPC_CLIENT, SLIPPAGE, TARGET_WALLET, ZSLOT_CLIENT
        },
        instructions::{
            buy_ix::BuyExactInInstructionAccountsExt, sell_ix::SellExactInInstructionAccountsExt,
        },
        service::Tips,
        utils::{
            blockhash::{get_slot, recent_blockhash_handler}, build_and_sign, sol_token_quote, token_sol_quote, TRADE_EVENT_DISC
        },
    },
    serde_json::json,
    solana_sdk::commitment_config::CommitmentConfig,
    solana_transaction_status_client_types::InnerInstruction,
    spl_associated_token_account::{
        get_associated_token_address, instruction::create_associated_token_account_idempotent,
    },
    std::{
        collections::{HashMap, HashSet},
        env,
        sync::Arc,
    },
    tokio::sync::RwLock,
    yellowstone_grpc_proto::geyser::{
        CommitmentLevel, SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
    },
};

#[tokio::main]
pub async fn main() -> CarbonResult<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    init_nozomi().await;
    init_zslot().await;
    init_jito().await;

    tokio::spawn({
        async move {
            loop {
                recent_blockhash_handler(RPC_CLIENT.clone()).await;
            }
        }
    });

    println!("TARGET_WALLET : {}", TARGET_WALLET.to_string());

    // NOTE: Workaround, that solving issue https://github.com/rustls/rustls/issues/1877
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Can't set crypto provider to aws_lc_rs");

    let transaction_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: Some(false),
        // account_include: vec![],
        account_include: vec![TARGET_WALLET.to_string()],
        account_exclude: vec![],
        account_required: vec![PUMPFUN_PROGRAM_ID.to_string().clone()],
        signature: None,
    };

    println!("Using payer: {}", *PUBKEY);

    let mut transaction_filters: HashMap<String, SubscribeRequestFilterTransactions> =
        HashMap::new();

    transaction_filters.insert(
        "jupiter_swap_transaction_filter".to_string(),
        transaction_filter,
    );

    let yellowstone_grpc = YellowstoneGrpcGeyserClient::new(
        env::var("GEYSER_URL").unwrap_or_default(),
        env::var("X_TOKEN").ok(),
        Some(CommitmentLevel::Processed),
        HashMap::new(),
        transaction_filters.clone(),
        Default::default(),
        Arc::new(RwLock::new(HashSet::new())),
    );

    let helius_laserstream = YellowstoneGrpcGeyserClient::new(
        env::var("LASER_ENDPOINT").unwrap_or_default(),
        env::var("LASER_TOKEN_KEY").ok(),
        Some(CommitmentLevel::Processed),
        HashMap::new(),
        transaction_filters.clone(),
        Default::default(),
        Arc::new(RwLock::new(HashSet::new())),
    );

    println!("Starting PUMPFUN Monitor...");

    carbon_core::pipeline::Pipeline::builder()
        .datasource(yellowstone_grpc)
        .datasource(helius_laserstream)
        .metrics(Arc::new(LogMetrics::new()))
        .metrics_flush_interval(3)
        .instruction(PumpfunDecoder, PumpfunInstructionProcessor)
        .shutdown_strategy(carbon_core::pipeline::ShutdownStrategy::Immediate)
        .build()?
        .run()
        .await?;

    println!("PUMPFUN Monitor has stopped.");

    Ok(())
}

pub struct PumpfunInstructionProcessor;

#[async_trait]
impl Processor for PumpfunInstructionProcessor {
    type InputType = InstructionProcessorInputType<PumpfunInstruction>;

    async fn process(
        &mut self,
        (metadata, instruction, nested_instructions, instructions): Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let signature = metadata.transaction_metadata.signature;

        let account_keys = metadata.transaction_metadata.message.static_account_keys();

        let instruction_clone: DecodedInstruction<PumpfunInstruction> = instruction.clone();

        let raw_instructions = match instruction.data {
            PumpfunInstruction::Buy(buy_data) => {
                println!("signature {:#?}", signature);

                if let Some(mut arranged) = Buy::arrange_accounts(&instruction_clone.accounts) {
                    arranged.user = *PUBKEY;
                    arranged.associated_user =
                        get_associated_token_address(&PUBKEY, &arranged.mint);

                    let inner_ixs: Vec<&InnerInstruction> = metadata
                        .transaction_metadata
                        .meta
                        .inner_instructions
                        .as_ref()
                        .and_then(|ixs| ixs.first())
                        .map(|ix_group| {
                            ix_group
                                .instructions
                                .iter()
                                .filter_map(|inner_ix| {
                                    let program_id_index =
                                        inner_ix.instruction.program_id_index as usize;
                                    let program_id = account_keys.get(program_id_index)?;

                                    let first_account_index =
                                        inner_ix.instruction.accounts.first()?;
                                    let first_account =
                                        account_keys.get(*first_account_index as usize)?;

                                    if *program_id == PUMPFUN_PROGRAM_ID
                                        && *first_account == arranged.event_authority
                                    {
                                        Some(inner_ix)
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let Some(swap_cpi_ix) = inner_ixs.first() else {
                        return Ok(()); // or Err(...) depending on your logic
                    };

                    if swap_cpi_ix
                        .instruction
                        .data
                        .starts_with(&TradeEvent::DISCRIMINATOR)
                    {
                        let trade_event =
                            TradeEvent::try_from_slice(&swap_cpi_ix.instruction.data[16..])
                                .expect("Failed to parse TradeEvent");

                        let required_token_amount = sol_token_quote(
                            *BUY_SOL_AMOUNT,
                            trade_event.virtual_sol_reserves,
                            trade_event.virtual_token_reserves,
                            true,
                        );

                        let lamports_with_slippage =
                            (*BUY_SOL_AMOUNT as f64 * 1.011 * (1.0 + *SLIPPAGE)) as u64;

                        println!("trade_event {:#?}", trade_event);

                        let create_ata_ix = arranged.get_create_idempotent_ata_ix();

                        let buy_ix = arranged.get_buy_ix(Buy {
                            amount: required_token_amount,
                            max_sol_cost: lamports_with_slippage,
                        });

                        vec![create_ata_ix, buy_ix]
                    } else {
                        vec![]
                    }
                } else {
                    println!("Failed to arrange accounts");

                    vec![]
                }
            }
            PumpfunInstruction::Sell(sell_data) => {
                println!("{:#?}", signature);

                if let Some(mut arranged) = Sell::arrange_accounts(&instruction_clone.accounts) {
                    arranged.user = *PUBKEY;
                    arranged.associated_user =
                        get_associated_token_address(&PUBKEY, &arranged.mint);

                    println!("{:#?}", account_keys);

                    let inner_ixs: Vec<&InnerInstruction> = metadata
                        .transaction_metadata
                        .meta
                        .inner_instructions
                        .as_ref()
                        .and_then(|ixs| ixs.first())
                        .map(|ix_group| {
                            ix_group
                                .instructions
                                .iter()
                                .filter_map(|inner_ix| {
                                    let program_id_index =
                                        inner_ix.instruction.program_id_index as usize;
                                    let program_id = account_keys.get(program_id_index)?;

                                    let first_account_index =
                                        inner_ix.instruction.accounts.first()?;
                                    let first_account =
                                        account_keys.get(*first_account_index as usize)?;

                                    if *program_id == PUMPFUN_PROGRAM_ID
                                        && *first_account == arranged.event_authority
                                    {
                                        Some(inner_ix)
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let Some(swap_cpi_ix) = inner_ixs.first() else {
                        return Ok(()); // or Err(...) depending on your logic
                    };

                    if swap_cpi_ix
                        .instruction
                        .data
                        .starts_with(&TradeEvent::DISCRIMINATOR)
                    {
                        let trade_event =
                            TradeEvent::try_from_slice(&swap_cpi_ix.instruction.data[16..])
                                .expect("Failed to parse TradeEvent");

                        let token_balance = match RPC_CLIENT
                            .get_token_account_balance_with_commitment(
                                &arranged.associated_user,
                                CommitmentConfig::confirmed(),
                            )
                            .await
                        {
                            Ok(response) => response.value.amount,
                            Err(e) => {
                                eprintln!("Failed to get token balance: {:?}", e);
                                return Ok(());
                            }
                        };

                        let token_amount = match token_balance.parse::<u64>() {
                            Ok(amount) => amount,
                            Err(e) => {
                                return Ok(());
                            }
                        };

                        let min_sol_amount_out = token_sol_quote(
                            token_amount,
                            trade_event.virtual_sol_reserves,
                            trade_event.virtual_token_reserves,
                            false,
                        );

                         let lamports_with_slippage =
                            (*BUY_SOL_AMOUNT as f64 * 1.011 * (1.0 - *SLIPPAGE)) as u64;

                        println!("trade_event {:#?}", trade_event);

                        let sell_ix = arranged.get_sell_ix(Sell {
                            amount: token_amount,
                            min_sol_output: lamports_with_slippage,
                        });

                        let close_ata_ix = arranged.get_close_ata_ix();

                        vec![sell_ix, close_ata_ix]
                    } else {
                        vec![]
                    }
                } else {
                    println!("Failed to arrange accounts");

                    vec![]
                }
            }
            _ => {
                vec![]
            }
        };

        if !raw_instructions.is_empty() {
            let (cu, priority_fee_micro_lamport, third_party_fee) = *PRIORITY_FEE;

            let results = match CONFIRM_SERVICE.as_str() {
                "NOZOMI" => {
                    let nozomi = NOZOMI_CLIENT.get().expect("Nozomi client not initialized");

                    let ixs = nozomi.add_tip_ix(Tips {
                        cu: Some(cu),
                        priority_fee_micro_lamport: Some(priority_fee_micro_lamport),
                        payer: *PUBKEY,
                        pure_ix: raw_instructions.clone(),
                        tip_addr_idx: 1,
                        tip_sol_amount: third_party_fee,
                    });

                    let recent_blockhash = get_slot();

                    let encoded_tx = build_and_sign(ixs, recent_blockhash, None);

                    match nozomi.send_transaction(&encoded_tx).await {
                        Ok(data) => json!({ "result": data }),
                        Err(err) => {
                            json!({ "result": "error", "message": err.to_string() })
                        }
                    }
                }
                "ZERO_SLOT" => {
                    let zero_slot = ZSLOT_CLIENT.get().expect("ZSlot client not initialized");

                    let ixs = zero_slot.add_tip_ix(Tips {
                        cu: Some(cu),
                        priority_fee_micro_lamport: Some(priority_fee_micro_lamport),
                        payer: *PUBKEY,
                        pure_ix: raw_instructions,
                        tip_addr_idx: 1,
                        tip_sol_amount: third_party_fee,
                    });

                    let recent_blockhash = get_slot();

                    let encoded_tx = build_and_sign(ixs, recent_blockhash, None);

                    match zero_slot.send_transaction(&encoded_tx).await {
                        Ok(data) => json!({ "result": data }),
                        Err(err) => {
                            json!({ "result": "error", "message": err.to_string() })
                        }
                    }
                }
                "JITO" => {
                    let jito = JITO_CLIENT.get().expect("Jito client not initialized");

                    let ixs = jito.add_tip_ix(Tips {
                        cu: Some(cu),
                        priority_fee_micro_lamport: Some(priority_fee_micro_lamport),
                        payer: *PUBKEY,
                        pure_ix: raw_instructions,
                        tip_addr_idx: 1,
                        tip_sol_amount: third_party_fee,
                    });

                    let recent_blockhash = get_slot();

                    let encoded_tx = build_and_sign(ixs, recent_blockhash, None);

                    match jito.send_transaction(&encoded_tx).await {
                        Ok(data) => json!({ "result": data }),
                        Err(err) => {
                            json!({ "result": "error", "message": err.to_string() })
                        }
                    }
                }
                _ => {
                    json!({ "result": "error", "message": "unknown confirmation service" })
                }
            };

            println!("TX HASH : {:#?}", results);
        };

        Ok(())
    }
}
