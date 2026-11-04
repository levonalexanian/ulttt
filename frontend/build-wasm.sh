#!/usr/bin/env sh
# Builds the Rust engine (backend/crates/wasm) to WebAssembly and generates its
# JavaScript bindings in src/wasm/.
set -eu
cd "$(dirname "$0")"
# Run cargo from backend/ so its rust-toolchain.toml (with the wasm target) applies.
(cd ../backend && cargo build --release --target wasm32-unknown-unknown -p ulttt-wasm)
wasm-bindgen --target web --out-dir src/wasm ../backend/target/wasm32-unknown-unknown/release/ulttt_wasm.wasm
