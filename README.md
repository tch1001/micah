# Uniswap Trades Tracker

[https://etherscan.io/address/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640#tokentxns](https://etherscan.io/address/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640#tokentxns)

## Setup
Tunnel to home network for eth full node.
```bash
ssh -N -f -L 8546:localhost:8546 -L 8545:localhost:8545 <REDACTED> 
cp .env.example .env # replace with own stuff
cargo run
```