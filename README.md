# 🚀 PumpFun Sniper Farm Bot

> **Automated token creation, monitoring, and selling on PumpFun**

A high-performance Rust bot that creates tokens on PumpFun, monitors for buy transactions, and automatically executes sell orders with MEV protection.

## ✨ Features

- 🎯 **Token Creation** - Deploy new tokens on PumpFun with custom metadata
- 👁️ **Real-time Monitoring** - Watch blockchain for buy transactions via Yellowstone gRPC
- ⚡ **Instant Selling** - Pre-created sell instructions for maximum speed
- 🛡️ **MEV Protection** - Multiple confirmation services (Jito, Nozomi, ZSlot)
- 🔄 **Auto-execution** - Sell tokens immediately when someone buys them

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Token         │    │   Real-time      │    │   Automated     │
│   Creation      │───▶│   Monitoring     │───▶│   Selling       │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   PumpFun       │    │   Yellowstone    │    │   MEV Services  │
│   Protocol      │    │   gRPC Stream    │    │   (Jito/Nozomi) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Solana CLI
- Wallet with SOL for gas fees

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd pumpfun-sniper-farm-bot

# Build the project
cargo build --release
```

### Configuration

Create a `.env` file:

```bash
# Required
PRIVATE_KEY=your_wallet_private_key_here
TARGET_WALLET=wallet_to_monitor_for_buys
RPC_ENDPOINT=https://api.mainnet-beta.solana.com
CONFIRM_SERVICE=JITO

# Token Settings
TOKEN_NAME=My Token
TOKEN_SYMBOL=MTK
TOKEN_URI=https://example.com/metadata.json
INITIAL_SOL_AMOUNT=0.001
SLIPPAGE=1.0

# Optional Services
NOZOMI_API_KEY=your_nozomi_key
ZERO_SLOT_KEY=your_zslot_key
GEYSER_URL=your_geyser_endpoint
LASER_ENDPOINT=your_laser_endpoint
```

### Run

```bash
cargo run --release
```

## 📋 How It Works

### 1. **Token Creation**
- Creates a new token on PumpFun with your specified metadata
- Makes an initial buy to establish liquidity
- Generates sell instructions and stores them globally

### 2. **Monitoring**
- Connects to Solana blockchain via Yellowstone gRPC
- Filters transactions for your target wallet and PumpFun program
- Detects when someone buys your created token

### 3. **Automated Selling**
- Instantly executes pre-created sell instructions
- Uses MEV protection services for fast confirmation
- Sells all tokens when a buy is detected

## ⚙️ Configuration Options

| Variable | Description | Default |
|----------|-------------|---------|
| `CONFIRM_SERVICE` | Confirmation service | `JITO` |
| `SLIPPAGE` | Slippage tolerance (%) | `1.0` |
| `BUY_SOL_AMOUNT` | Initial buy amount (SOL) | `0.001` |
| `PRIORITY_FEE` | Priority fee (micro lamports) | `0` |

### Confirmation Services

- **JITO** - MEV protection with tip-based priority
- **NOZOMI** - Alternative fast confirmation service  
- **ZERO_SLOT** - Zero-slot confirmation for maximum speed

## 🔧 Development

### Project Structure

```
src/
├── main.rs              # Main application entry point
├── config/              # Configuration management
│   ├── credentials.rs   # API keys and wallet setup
│   ├── trade_setting.rs # Trading parameters
│   └── clients.rs       # Service client initialization
├── instructions/        # PumpFun instruction builders
│   ├── pumpfun_buy.rs   # Token creation logic
│   └── pumpfun_sell.rs  # Selling functionality
├── service/             # External service integrations
│   ├── jito/           # Jito MEV protection
│   ├── nozomi/         # Nozomi confirmation
│   └── zero_slot/      # ZSlot confirmation
└── utils/              # Utility functions
```

### Key Components

- **Carbon Framework** - Blockchain data processing
- **Yellowstone gRPC** - Real-time transaction streaming
- **PumpFun Decoder** - Protocol-specific instruction parsing

## 📊 Performance

- **Sub-second Response** - Pre-created instructions for instant execution
- **Low Latency** - Direct gRPC streaming for real-time updates
- **High Throughput** - Async Rust for concurrent processing
- **Memory Efficient** - Static variables for global state management

## 📞 Support [https://t.me/Rust0x_726]

For questions and support:
- Open an issue on GitHub
- Check the documentation
- Review the code examples

---

**Happy Trading! 🎯**
