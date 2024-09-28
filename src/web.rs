mod process;
use core::hash;
use std::error::Error;
use std::{net::TcpListener, sync::Arc};

use alloy::providers::ProviderBuilder;
use axum::extract::{Path, Query};
use axum::routing::get;
use axum::{extract::State, Router};
use process::{binance_eth_usdt_price, decode_tx};
use rdkafka::producer::FutureProducer;
use redis::AsyncCommands;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::Mutex;

use bb8_redis::{bb8, RedisConnectionManager};

async fn health() -> &'static str {
    "OK"
}
async fn get_tx(
    State(pool): State<bb8::Pool<RedisConnectionManager>>,
    Path(hash): Path<String>,
) -> Result<String, (StatusCode, String)> {
    let rpc_url_http = std::env::var("RPC_URL_HTTP").expect("RPC_URL_HTTP must be set");
    let provider =
        ProviderBuilder::new().on_http(rpc_url_http.parse().expect("Invalid RPC_URL_HTTP"));
    let mut conn = pool.get().await.map_err(|e| {
        println!("Failed to get connection: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    })?;

    // let hash = "0xd01a5063a485cee4045fb6edad8a72329680604b5e4e62327b68aa470cd4c65c";
    let hash = hash.as_str();
    // check for cache first
    let cached = conn.get(hash).await.map_err(|e| {
        println!("Failed to get key: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    if let Some(cached) = cached {
        return Ok(cached);
    }

    // not in cache, need to compute
    let (gas_fee, timestamp_sec) = match decode_tx(&provider, &hash.to_string()).await {
        Err(e) => {
            println!("Ignoring {}, Error: {}", hash, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
        Ok((_gas_fee, _timestamp)) => {
            println!("Decoded tx, {}, {}", _gas_fee, _timestamp);
            (_gas_fee, _timestamp)
        }
    };

    let key = hash;
    let value = match binance_eth_usdt_price(timestamp_sec * 1000).await {
        Err(e) => {
            println!("Error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
        Ok((_closest_time, _price)) => {
            println!("Got price: {}, {}", _closest_time, _price);
            let gas_fee_usdt = gas_fee as f64 * _price / 1e18;
            gas_fee_usdt.to_string()
        }
    };
    // save to cache
    conn.set::<&str, &str, ()>(key, &value).await.map_err(|e| {
        println!("Failed to set key: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    })?;
    Ok(value)
}

async fn erc20_token_transfer_events(
    start_block: u64,
    end_block: u64,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = "https://api.etherscan.io/api";
    let module = "account";
    let action = "tokentx";
    // let contractaddress = "0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2";
    let address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
    let page = "1";
    let offset = "100";
    // let startblock = "0";
    // let endblock = "27025780";
    let sort = "desc";
    let apikey = std::env::var("ETHERSCAN_API_KEY").map_err(|e| {
        println!("Failed to get ETHERSCAN_API_KEY: {}", e);
        return Box::new(e) as Box<dyn std::error::Error>;
    })?;
    let url = format!(
        "{}?module={}&action={}&address={}&page={}&offset={}&sort={}&apikey={}",
        url, module, action, address, page, offset, sort, apikey
    );
    println!("url = {}", url);
    let resp = reqwest::get(url).await.map_err(|e| {
        println!("Failed to get url: {}", e);
        return Box::new(e) as Box<dyn std::error::Error>;
    })?;
    let resp = resp.json::<serde_json::Value>().await.map_err(|e| {
        println!("Failed to parse json: {}", e);
        return Box::new(e) as Box<dyn std::error::Error>;
    })?;
    let resp = resp["result"].as_array().ok_or("No result")?;
    let mut vec = Vec::new();
    for entry in resp.iter() {
        let hash = entry["hash"].as_str().ok_or("No hash")?;
        vec.push(hash.to_string());
    }
    Ok(vec)
}

#[derive(Deserialize)]
struct BatchParams {
    start_block: u64,
    end_block: u64,
}

async fn process_batch(
    State(pool): State<bb8::Pool<RedisConnectionManager>>,
    Query(params): Query<BatchParams>,
) -> Result<String, (StatusCode, String)> {
    let hashes = erc20_token_transfer_events(params.start_block, params.end_block)
        .await
        .map_err(|e| {
            println!("Error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        })?;

    let kafka_broker = std::env::var("KAFKA_BROKER").map_err(|e| {
        println!("Failed to get KAFKA_BROKER: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    })?;
    let producer: FutureProducer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", kafka_broker.as_str())
        .create()
        .map_err(|e| {
            println!("Failed to create producer: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        })?;
    let topic = std::env::var("TOPIC").map_err(|e| {
        println!("Failed to get TOPIC: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    })?;
    for hash in hashes.iter() {
        producer
            .send(
                rdkafka::producer::FutureRecord::to(topic.as_str())
                    .payload(hash.as_str())
                    .key(hash.as_str()),
                std::time::Duration::from_secs(0),
            )
            .await
            .map_err(|e| {
                println!("Failed to send, kafka error");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Kafka error".to_string());
            })?;
    }

    // return json: {ok: true, hashes: [hash1, hash2, ...]}
    return Ok(serde_json::json!({ "ok": true, "hashes": hashes }).to_string());
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let manager = RedisConnectionManager::new(redis_url).expect("Failed to create manager");
    let pool = bb8::Pool::builder()
        .build(manager)
        .await
        .expect("Failed to create pool");
    {
        let mut conn = pool.get().await.expect("Failed to get connection");
        conn.set::<&str, &str, ()>("test", "test")
            .await
            .expect("Failed to set key");
        let result: String = conn.get("test").await.expect("Failed to get key");
        assert_eq!(result, "test");
    }
    let app = Router::new().route("/health", get(health));
    let app = app.route("/batch", get(process_batch));
    let app = app.route("/tx/:hash", get(get_tx)).with_state(pool);
    // batch?start_time=123&end_time=456
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7878")
        .await
        .expect("Failed to bind to port");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Failed to serve using axum");
}
