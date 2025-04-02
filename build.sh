# format, test, build
cargo +nightly fmt
cargo t -p pallet-multisig
cargo build
# cargo build --release