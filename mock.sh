# cargo run --release -- -k 21 --function zkmain --wasm wasm/zkdex_wasm_mock_bg_opt.wasm dry-run

## funding 100, 33ms
# cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_funding_100.wasm dry-run
#
## oracle 100, 18ms
# cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_oracle_100.wasm dry-run
#
## deposit 100, 37ms
# cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_deposit_100.wasm dry-run

# trade 1, 170ms
RUST_BACKTRACE=1 cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_trade_100.wasm dry-run

