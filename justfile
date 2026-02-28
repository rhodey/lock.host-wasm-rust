wkg:
  cargo install wkg
  wkg wit fetch

helpers:
  npm --prefix helpers install
  npm --prefix helpers run build

plug:
  cp target/wasm32-wasip2/release/lock_host_wasm_rust.wasm target/wasm32-wasip2/release/total.wasm

build:
  just helpers
  rustup target add wasm32-wasip2
  cargo build --release
  just plug

env:
  awk '!/^\s*#/ && NF { printf "--env %s ", $$0 }' .env

run:
  wasmtime serve -Scli -Shttp target/wasm32-wasip2/release/total.wasm $(just env)

joke joke:
  curl -G -d "addr=CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W" --data-urlencode "message={{joke}}" http://localhost:8080/api/joke && echo
