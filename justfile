build:
    npm install
    npm run build

env:
    awk '!/^\s*#/ && NF { printf "--env %s ", $$0 }' .env

run:
    wasmtime serve -S common dist/*.wasm $(just env)

joke joke:
    curl -G -d "addr=CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W" --data-urlencode "message={{joke}}" http://localhost:8080/api/joke && echo
