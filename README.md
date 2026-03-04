# Lock.host-wasm-rust
Lock.host WASM Rust example, see [Lock.host](https://github.com/rhodey/lock.host)

This demonstration uses OpenAI to control a Solana wallet:
+ OpenAI API calls
+ Solana API calls
+ Hit /api/joke?message=your best joke&addr=abc123
+ OAI is asked "You are to decide if a joke is funny or not"
+ If so 0.001 SOL is sent to addr
+ jokes are written to SQLite by [SQLiteWasmWasi](https://github.com/rhodey/sqlitewasmwasi)

## Why
[Lock.host-node](https://github.com/rhodey/lock.host-node) demonstrates the same features but is expensive to host

[Lock.host-python](https://github.com/rhodey/lock.host-python) also demonstrates the same features and is expensive to host

It is very efficient to host WASM apps and so Lock.host has started in this direction

## Setup
Install [just](https://github.com/casey/just) then [wasmtime](https://github.com/bytecodealliance/wasmtime):
```
apt install just (or brew install just)
curl https://wasmtime.dev/install.sh -sSf | bash
```

You also need [wac](https://github.com/bytecodealliance/wac) and this one takes a few minutes:
```
cargo install wac-cli
```

## Run
+ [http://localhost:8080](http://localhost:8080)
+ [app wallet](https://explorer.solana.com/address/DohcaGiBiC3yuPz4gHtoA7QJhyL5N7hk3EpnfFyHZR2S?cluster=devnet)
+ [user wallet](https://explorer.solana.com/address/CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W?cluster=devnet)
```
cp example.env .env
just build
just run
just joke 'why did the worker quit his job at the recycling factory? because it was soda pressing.'
> {"from":"DohcaGiBiC3yuPz4gHtoA7QJhyL5N7hk3EpnfFyHZR2S","signature":"2DF5yVe1dHoTa51RCDUDHzWGnNGbQVsmfieiQRn3hcYgADX4u8rezGrbVhfc4MwWKTiBBqjwaSGHkaueuzGTVXvq","thoughts":"The play on words with 'soda pressing' and 'so depressing' is clever and adds a humorous twist, making it a fun pun.","to":"CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W"}
sqlite3 mount/app.db "select * from jokes;"
> 1|CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W|why did the worker quit his job at the recycling factory? because it was soda pressing.|The play on words with 'soda pressing' and 'so depressing' is clever and adds a humorous twist, making it a fun pun.|1
```

## Notes
The Rust target [wasm32-wasip2](https://doc.rust-lang.org/nightly/rustc/platform-support/wasm32-wasip2.html) is getting more support everyday but still many crates are not allowed

Additionally many crates do not support [async features](https://doc.rust-lang.org/book/ch17-00-async-await.html) and should not be considered for server use

The use of OpenAI and Solana in this example does not involve friendly crates-- but is async!

[SQLiteWasmWasi](https://github.com/rhodey/sqlitewasmwasi) is using [rusqlite](https://crates.io/crates/rusqlite) internally

## Performance
1. npx loadtest -n 10000 http://localhost:8080 == 14620 RPS
2. npx loadtest -n 10000 -k http://localhost:8080 == 22075 RPS

## License
hello@lock.host

MIT
