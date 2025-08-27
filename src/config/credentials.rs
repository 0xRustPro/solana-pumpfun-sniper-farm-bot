use dotenvy::dotenv;
use once_cell::sync::Lazy;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};
use std::{env, panic, str::FromStr, sync::Arc};

fn try_parse_keypair(input: &str) -> Result<Keypair, String> {
    // Try JSON array of bytes
    if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(input) {
        return Keypair::from_bytes(&bytes).map_err(|e| format!("invalid keypair bytes: {}", e));
    }

    // Try base64-encoded bytes
    if let Ok(bytes) = base64::decode(input) {
        return Keypair::from_bytes(&bytes).map_err(|e| format!("invalid base64 keypair: {}", e));
    }

    // Try base58-encoded keypair (guard against panics)
    match panic::catch_unwind(|| Keypair::from_base58_string(input)) {
        Ok(kp) => Ok(kp),
        Err(_) => Err(
            "unsupported PRIVATE_KEY format; expected base58 (64-byte), JSON [u8;64], or base64"
                .to_string(),
        ),
    }
}

fn load_keypair_from_env() -> Keypair {
    dotenv().ok();

    let private_key = match env::var("PRIVATE_KEY") {
        Ok(v) => v,
        Err(_) => {
            eprintln!(
                "Error: PRIVATE_KEY must be set (base58 64-byte keypair, JSON [u8;64], or base64)"
            );
            std::process::exit(1);
        }
    };

    match try_parse_keypair(&private_key) {
        Ok(kp) => kp,
        Err(why) => {
            eprintln!("Error: Invalid PRIVATE_KEY: {}", why);
            std::process::exit(1);
        }
    }
}

pub static PRIVATE_KEY: Lazy<Keypair> = Lazy::new(load_keypair_from_env);
pub static PUBKEY: Lazy<Pubkey> = Lazy::new(|| PRIVATE_KEY.pubkey());

pub static TARGET_WALLET: Lazy<Pubkey> = Lazy::new(|| {
    dotenv().ok();

    let target_wallet = match env::var("TARGET_WALLET") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: TARGET_WALLET must be set (base58 pubkey)");
            std::process::exit(1);
        }
    };

    match Pubkey::from_str(&target_wallet) {
        Ok(pk) => pk,
        Err(_) => {
            eprintln!("Error: TARGET_WALLET must be a valid base58 pubkey");
            std::process::exit(1);
        }
    }
});

pub static RPC_ENDPOINT: Lazy<String> = Lazy::new(|| {
    dotenv().ok();

    let rpc_endpoint = env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set");

    rpc_endpoint
});

pub static RPC_CLIENT: Lazy<Arc<RpcClient>> = Lazy::new(|| {
    dotenv().ok();

    let rpc_endpoint = env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set");

    Arc::new(RpcClient::new_with_commitment(
        rpc_endpoint,
        CommitmentConfig::processed(),
    ))
});

pub static LASER_ENDPOINT: Lazy<String> = Lazy::new(|| {
    dotenv().ok();

    let laser_endpoint = env::var("LASER_ENDPOINT").expect("LASER_ENDPOINT must be set");

    laser_endpoint
});

pub static LASER_TOKEN_KEY: Lazy<String> = Lazy::new(|| {
    dotenv().ok();

    let laser_token_key = env::var("LASER_TOKEN_KEY").expect("LASER_TOKEN_KEY must be set");

    laser_token_key
});

pub static GRPC_ENDPOINT: Lazy<String> = Lazy::new(|| {
    dotenv().ok();

    let grpc_endpoint = env::var("GRPC_ENDPOINT").expect("GRPC_ENDPOINT must be set");

    grpc_endpoint
});

pub static GRPC_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();

    let grpc_token = env::var("GRPC_TOKEN").expect("GRPC_TOKEN must be set");

    grpc_token
});
