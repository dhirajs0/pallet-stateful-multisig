# format, test, build
cargo +nightly fmt
RUST_BACKTRACE=1 cargo t -p pallet-multisig
cargo check
# cargo build --release