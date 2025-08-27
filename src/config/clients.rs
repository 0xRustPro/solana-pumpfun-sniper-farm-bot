use dotenvy::dotenv;
use std::env;
use tokio::sync::OnceCell;

use crate::service::{Jito, Nozomi, ZeroSlot};

pub static NOZOMI_CLIENT: OnceCell<Nozomi> = OnceCell::const_new();
pub static ZSLOT_CLIENT: OnceCell<ZeroSlot> = OnceCell::const_new();
pub static JITO_CLIENT: OnceCell<Jito> = OnceCell::const_new();

pub async fn init_nozomi() {
    dotenv().ok();

    let nozomi_api_key = env::var("NOZOMI_API_KEY").expect("NOZOMI_API_KEY not set in .env");

    let nozomi = Nozomi::new_auto(nozomi_api_key).await;
    nozomi.health_check(50);
    NOZOMI_CLIENT.set(nozomi).unwrap();
}

pub async fn init_zslot() {
    dotenv().ok();

    let zslot_api_key = env::var("ZERO_SLOT_KEY").expect("ZERO_SLOT_KEY not set in .env");

    let zslot = ZeroSlot::new_auto(zslot_api_key).await;
    ZSLOT_CLIENT.set(zslot).unwrap();
}

pub async fn init_jito() {
    let jito = Jito::new_auto(None).await;
    JITO_CLIENT.set(jito).unwrap();
}

