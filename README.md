# Paint my NFT

Heavily inspired from this example: https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs

## Quick start

Start redis
```bash
docker run --rm -p 6379:6379 --name paint-game redis
```

Start the server
```bash
cargo build
RUST_LOG=debug cargo run --bin game
```

Go to http://127.0.0.1:3030/static/index.html on multiple tabs

Delete stream
```bash
docker exec paint-game redis-cli del paint-game
```

Run simulated users
```bash
cargo run --bin simulated_users
```
