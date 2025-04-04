#!/bin/bash

# Exit on error
set -e

# Format code
cargo +nightly fmt

# Run clippy
cargo +nightly clippy

# Run tests
RUST_BACKTRACE=1 cargo test -p pallet-multisig

# Build the runtime in release mode
cargo build -p pba-runtime --release

echo "Build completed successfully!"


# polkadot-omni-node --chain chain_spec.json --dev-block-time 6000 --tmp