# format, test, build
cargo +nightly fmt
cargo +nightly clippy
# taplo format --config .config/taplo.toml 
RUST_BACKTRACE=1 cargo t -p pallet-multisig
cargo build
# cargo build --release