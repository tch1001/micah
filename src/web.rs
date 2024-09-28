mod process;
use std::{net::TcpListener, sync::Arc};

use alloy::providers::ProviderBuilder;
use process::{binance_eth_usdt_price, decode_tx};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let rpc_url_http = std::env::var("RPC_URL_HTTP").expect("RPC_URL_HTTP must be set");
    let topic = std::env::var("TOPIC").expect("TOPIC must be set");

    let http_provider =
        ProviderBuilder::new().on_http(rpc_url_http.parse().expect("Invalid RPC_URL_HTTP"));
    let http_provider = Arc::new(Mutex::new(http_provider));
    let redis_client =
        redis::Client::open("redis://localhost:6379").expect("Failed to connect to Redis");
    let mut redis_con = redis_client
        .get_connection()
        .expect("Failed to get connection");
    let redis_con = Arc::new(Mutex::new(redis_con));
    let listener = TcpListener::bind("0.0.0.0:7878").expect("Failed to bind to port 7878");
    for stream in listener.incoming() {
        let http_provider = http_provider.clone();
        let mut redis_con = redis_con.clone();
        tokio::spawn(async move {
            let stream = stream.expect("Failed to get stream");
            process::handle_connection(stream, http_provider, &mut redis_con).await;
        });
    }
}
