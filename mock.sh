RUST_BACKTRACE=1 RUST_LOG=info cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/zkdex_mock_no_trade.wasm dry-run | grep "time elapse:"

RUST_BACKTRACE=1 RUST_LOG=info cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/zkdex_mock_origin.wasm dry-run | grep "time elapse:"

RUST_BACKTRACE=1 RUST_LOG=info cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/zkdex_mock_100_deposit.wasm dry-run | grep "time elapse:"
