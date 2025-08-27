pub fn sol_token_quote(
    amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    is_buy: bool,
) -> u64 {
    let out_token_amount;
    if is_buy {
        out_token_amount = virtual_token_reserves as f64
            / (amount as f64 + virtual_sol_reserves as f64)
            * (amount as f64);
    } else {
        out_token_amount = virtual_token_reserves as f64
            / (amount as f64 + virtual_sol_reserves as f64 - 1.0)
            * (amount as f64 + 1.0);
    }

    out_token_amount as u64
}

pub fn token_sol_quote(
    amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    is_buy: bool,
) -> u64 {
    let out_sol_amount;
    if is_buy {
        out_sol_amount = amount as f64 / (virtual_token_reserves as f64 - amount as f64)
            * virtual_sol_reserves as f64;
    } else {
        out_sol_amount = amount as f64 / (virtual_token_reserves as f64 + amount as f64)
            * virtual_sol_reserves as f64;
    }

    out_sol_amount as u64
}