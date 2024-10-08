use std::error::Error;

use alloy::{
    eips::BlockId,
    primitives::TxHash,
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::BlockTransactionsKind,
};
use rdkafka::{
    admin::AdminOptions,
    config::FromClientConfig,
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message,
};
use redis::{Commands, Connection, RedisError};

pub async fn decode_tx(
    provider: &RootProvider<alloy::transports::http::Http<reqwest::Client>>,
    hash: &String,
) -> Result<(u128, u64), Box<dyn std::error::Error>> {
    // let provider = *provider;
    let hash: TxHash = hash.parse().unwrap();
    let receipt = provider.get_transaction_receipt(hash);
    let receipt = receipt
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or_else(|| "No Receipt".to_string())
        .map_err(|e| format!("Error {}", e))?;

    let tx = provider
        .get_transaction_by_hash(hash)
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or_else(|| format!("No Transaction"))
        .map_err(|e| format!("Error {}", e))?;
    let block_number = tx.block_number.ok_or("No block number")?;
    let gas_price = tx.gas_price.ok_or("No gas price")?;
    let gas_used = receipt.gas_used;
    let gas_fee = gas_price * gas_used;
    let block_id: BlockId = block_number.into();

    let timestamp = provider
        .get_block(block_id, BlockTransactionsKind::Hashes)
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or("No block")?
        .header
        .timestamp;

    Ok((gas_fee, timestamp))
}

pub async fn binance_eth_usdt_price(time_ms: u64) -> Result<(u64, f64), Box<dyn Error>> {
    // get the price over 10s
    let mut duration_range: u64 = 1000;
    for i in 0..10 {
        duration_range *= 2;
        let mut url: String = "https://api.binance.com/api/v3/aggTrades?symbol=ETHUSDT"
            .parse()
            .unwrap();
        url += "&startTime=";
        url += (time_ms - duration_range).to_string().as_str();
        url += "&endTime=";
        url += (time_ms + duration_range).to_string().as_str();
        println!("url = {}", url);
        let resp = reqwest::get(url).await.unwrap();
        let resp = resp.json::<serde_json::Value>().await.unwrap();
        // println!("resp = {:?}", resp);
        if resp.as_array().unwrap().len() == 0 {
            println!("no data found for duration_range = {}", duration_range);
        } else {
            let resp = resp.as_array().unwrap();
            let mut closest_time = resp[0]["T"].as_u64().unwrap();
            let mut returned_price = resp[0]["p"].as_str().unwrap().parse::<f64>().unwrap();
            for entry in resp[1..].iter() {
                let data: &serde_json::Map<String, serde_json::Value> = entry.as_object().unwrap();
                let entry_time: u64 = data["T"].as_u64().unwrap();
                let entry_price: f64 = data["p"].as_str().unwrap().parse::<f64>().unwrap();
                let delta_time = if entry_time < time_ms {
                    time_ms - entry_time
                } else {
                    entry_time - time_ms
                };
                let delta_closest_time = if closest_time < time_ms {
                    time_ms - closest_time
                } else {
                    closest_time - time_ms
                };
                if delta_time < delta_closest_time {
                    closest_time = entry_time;
                    returned_price = entry_price;
                }
            }
            return Ok((closest_time, returned_price));
        }
    }
    Err("No data found".into())
}

pub async fn send_to_cache(
    redis: &mut Connection,
    key: &str,
    value: &str,
) -> Result<bool, RedisError> {
    match redis.set::<&str, &str, ()>(key, value) {
        Ok(_) => return Ok(true),
        Err(e) => {
            return Err(e);
        }
    }
}
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let rpc_url_http = std::env::var("RPC_URL_HTTP").expect("RPC_URL_HTTP must be set");
    let topic = std::env::var("TOPIC").expect("TOPIC must be set");

    // let topic = "test-topic";
    let kafka_broker = std::env::var("KAFKA_BROKER").expect("KAFKA_BROKER must be set");
    println!("Kafka Broker: {}", kafka_broker);
    let group_id = std::env::var("GROUP_ID").map_or("test-group".to_string(), |x| x);

    // // create a topic if it doesn't exist
    // let admin_client = rdkafka::admin::AdminClient::from_config(
    //     &ClientConfig::new().set("bootstrap.servers", kafka_broker.as_str()),
    // )
    // .expect("AdminClient creation failed");
    // let topic_spec = rdkafka::admin::NewTopic::new(
    //     topic.as_str(),
    //     1,
    //     rdkafka::admin::TopicReplication::Fixed(1),
    // );
    // let _ = admin_client
    //     .create_topics(&[topic_spec], &AdminOptions::new())
    //     .await
    //     .expect("Failed to create topic");

    let consumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", kafka_broker.as_str())
        .create::<StreamConsumer>()
        .expect("Consumer creation failed");
    consumer
        .subscribe(&[topic.as_str()])
        .expect("Can't subscribe to specified topic");

    let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> =
        ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let client = redis::Client::open(redis_url).expect("Failed to connect to Redis");
    let mut con = client.get_connection().expect("Failed to get connection");

    loop {
        let message = consumer.recv().await.expect("Consumer failure");
        let message = message.detach();
        let message = message.payload();
        let mut hash = match message {
            None => {
                println!("No message");
                continue;
            }
            Some(payload) => {
                let content = std::str::from_utf8(payload).expect("Invalid UTF-8");
                content.to_string()
            }
        };
        let (gas_fee, timestamp_sec) = match decode_tx(&http_provider, &hash).await {
            Err(e) => {
                println!("Ignoring {}, Error: {}", hash, e);
                continue;
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
                continue;
            }
            Ok((_closest_time, _price)) => {
                println!("Got price: {}, {}", _closest_time, _price);
                let gas_fee_usdt = gas_fee as f64 * _price / 1e18;
                gas_fee_usdt.to_string()
            }
        };
        loop {
            // TODO exponential backoff and DLQ
            match send_to_cache(&mut con, key.as_str(), value.as_str()).await {
                Ok(_) => {
                    println!("Sent to cache");
                    break;
                }
                Err(e) => {
                    println!("Redis Error: {}", e);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_decode_tx() {
        dotenv::dotenv().ok();

        let rpc_url_http = std::env::var("RPC_URL_HTTP").expect("RPC_URL_HTTP must be set");
        let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> =
            ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
        let hash = "0xd01a5063a485cee4045fb6edad8a72329680604b5e4e62327b68aa470cd4c65c";
        let result = decode_tx(&http_provider, &hash.to_string()).await;
        assert_eq!(result.is_ok(), true);
        let expected = (4297515337306724, 1727432927);
        assert_eq!(result.unwrap(), expected);
    }
}
