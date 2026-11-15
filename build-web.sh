#!/usr/bin/env sh
# Builds the browser version into web/pkg.
# Requires: rustup target add wasm32-unknown-unknown
#           cargo install wasm-bindgen-cli --version <same as Cargo.lock>
set -eu
cd "$(dirname "$0")"

cargo build -p romoku-web --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/romoku_web.wasm

echo "Built web/pkg. Serve with:  python3 -m http.server -d web 8000"
