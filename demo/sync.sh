#!/bin/bash

rm -rf pkg/*
scp -P 22 root@52.220.6.201:/root/zkdex-wasm-poc/core/lib/zkdex/pkg/zkdex_wasm_bg_opt.wasm /Users/lvcong/rust/zkWasm/demo/pkg/