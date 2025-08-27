# Simple Token Creator Example

This example demonstrates how to use the `get_create_buy_instruction` function to create and buy tokens in PumpFun in one transaction.

## How to Use

1. **Set environment variables:**
```bash
export PRIVATE_KEY="your_private_key_here"
export TOKEN_NAME="My Awesome Token"
export TOKEN_SYMBOL="MAT"
export TOKEN_URI="https://example.com/metadata.json"
export INITIAL_SOL_AMOUNT="0.1"
export RPC_ENDPOINT="https://api.mainnet-beta.solana.com"
```

2. **Run the example:**
```bash
cd examples
cargo run --bin simple-token-creator
```

## What It Does

The `get_create_buy_instruction` function creates 3 instructions:
1. **Token Creation** - Creates the token on PumpFun
2. **ATA Creation** - Creates your token account
3. **Initial Buy** - Adds liquidity by buying tokens

All in one atomic transaction!

## Key Benefits

- Single transaction for token creation + initial liquidity
- Uses bonding curve math for fair pricing
- Handles all complex instruction building
- Comprehensive error handling
