# cargo run --release -- -k 21 --function zkmain --wasm wasm/zkdex_wasm_mock_bg_opt.wasm dry-run

# funding 100, 2.83s
cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_funding_100.wasm dry-run

# oracle 100, 972ms
cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_oracle_100.wasm dry-run

# deposit 100, 3.64s
cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_deposit_100.wasm dry-run

# trade 1, 381ms
cargo run --release -- -k 21 --function zkmain --output ./output --wasm wasm/mock_trade_1.wasm dry-run

#bf1851a
