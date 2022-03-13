# Paint my NFT

Heavily inspired from this example: https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs

## Quick start

```bash
export REDIS_HOSTNAME=localhost:6379
```

```rust
cargo build
RUST_LOG="debug" cargo run
```
Go to http://127.0.0.1:3030/static/index.html on multiple tabs

```bash
docker run --rm -p 6379:6379 --name paint-game redis
```

