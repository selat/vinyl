# Vinyl

Vinyl is a reliable listener for EVM blockchains, including the Ethereum mainnet and any Ethereum-like blockchains, such as Polygon, Binance Smart Chain, and others. With Vinyl, you can seamlessly synchronize the state of an EVM blockchain locally and handle any reverts correctly.

The tool ensures that the state of the EVM blockchain is correctly reflected in your local state, taking into account possible chain reorganizations and state reverts.

## Features

- **Reliable listening:** Vinyl listens for new blocks and reliably processes them.

- **Correct handling of reverts:** In case of a blockchain reorganization, Vinyl correctly handles the reverts and updates the local state to reflect the state of the longest chain.

## Usage

```bash
# Clone the repository
git clone https://github.com/selat/vinyl.git
cd vinyl

# Define websocket RPC URL
export RPC_URL=wss://ws-matic-mainnet.chainstacklabs.com

# Run the project
cargo run --release
```

