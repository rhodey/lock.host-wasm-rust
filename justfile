setup:
  rustup target add wasm32-wasip2
  cargo install wkg
  wkg wit fetch

build:
  cargo build --release

env:
  awk '!/^\s*#/ && NF { printf "--env %s ", $$0 }' .env

run:
  wasmtime serve -Scli -Shttp target/wasm32-wasip2/release/*.wasm $(just env)

joke joke:
  curl -G -d "addr=CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W" --data-urlencode "message={{joke}}" http://localhost:8080/api/joke && echo
