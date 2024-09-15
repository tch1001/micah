# Uniswap Trades Tracker

[https://etherscan.io/address/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640#tokentxns](https://etherscan.io/address/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640#tokentxns)

## Setup

Tunnel to home network for eth full node.

```bash
ssh -N -f -L 8546:localhost:8546 -L 8545:localhost:8545 <REDACTED>
cp .env.example .env # replace with own stuff
cargo run
```

## Rust C++ FFI

We call rust from c++. This is done using a custom [build.rs](./build.rs).

```bash
cargo build
g++ target/cxxbridge/dashboard/src/main.rs.cc src/webserver.cpp -o meow -L./target/debug -ldashboard -I ./target/cxxbridge/dashboard/
LD_LIBRARY_PATH=./target/debug ./meow
```
