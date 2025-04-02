# format, test, build
cargo +nightly fmt
RUST_BACKTRACE=1 cargo t -p pallet-multisig
cargo build
# cargo build --release