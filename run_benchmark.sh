#!/bin/bash

# run the build script
./build.sh

# Build the runtime with benchmarks enabled
cargo build -p pba-runtime --release --features runtime-benchmarks

# Ensure the output directory exists
mkdir -p ./pallets/multisig/src/

# Check that the wasm file exists
if [ ! -f target/release/wbuild/pba-runtime/pba_runtime.compact.compressed.wasm ]; then
  echo "Error: Wasm file not found at target/release/wbuild/pba-runtime/pba_runtime.compact.compressed.wasm"
  echo "Please build your runtime first."
  exit 1
fi


# Run the benchmark command with genesis configuration
frame-omni-bencher v1 benchmark pallet \
  --runtime target/release/wbuild/pba-runtime/pba_runtime.compact.compressed.wasm \
  --pallet "pallet-multisig" \
  --extrinsic "*" \
  --output ./pallets/multisig/src/weights.rs \
  --genesis-builder none 
#   --template ./.maintain/frame-weight-template.hbs


