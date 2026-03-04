build:
  just helpers
  rustup target add wasm32-wasip2
  cargo build --release
  cp ~/.cargo/registry/src/index.crates.io-*/sqlite-wasm-wasi-*/component.wasm target/wasm32-wasip2/release/sqlite.wasm
  just plug

helpers:
  npm --prefix helpers install
  npm --prefix helpers run build

plug:
  wac plug \
    target/wasm32-wasip2/release/lock_host_wasm_rust.wasm \
    --plug helpers/dist/bundle.wasm \
    --plug target/wasm32-wasip2/release/sqlite.wasm \
    -o target/wasm32-wasip2/release/total.wasm

env:
  awk '!/^\s*#/ && NF { printf "--env %s ", $$0 }' .env

run:
  mkdir -p mount/
  wasmtime serve -S cli -S http --dir ./mount::/app target/wasm32-wasip2/release/total.wasm $(just env)

joke joke:
  curl -G -d "addr=CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W" --data-urlencode "message={{joke}}" http://localhost:8080/api/joke && echo
