use alloy::{eips::BlockId, primitives::TxHash, providers::{Provider, ProviderBuilder, RootProvider}, rpc::types::BlockTransactionsKind};
use rdkafka::{consumer::{Consumer, StreamConsumer}, ClientConfig, Message};
use redis::{Commands, Connection, RedisError};


async fn decode_tx(&provider: &RootProvider<alloy::transports::http::Http<reqwest::Client>>, hash: &String) 
    -> Result<(String, String), Box<dyn std::error::Error>> {
    // let provider = *provider;
    let hash: TxHash = hash.parse().unwrap();
    let receipt = provider.get_transaction_receipt(hash);
    let receipt = receipt
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or(|e| format!("No Receipt: {:?}", e))
        .map_err(|e| format!("Error"))?;

    let tx = provider.get_transaction_by_hash(hash)
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or(|e| format!("No Transaction: {:?}", e))
        .map_err(|e| format!("Error"))?;
    let block_number = tx.block_number.ok_or("No block number")?;
    let gas_price = tx.gas_price.ok_or("No gas price")?;
    let gas_used = receipt.gas_used;
    let gas_fee = gas_price * gas_used;
    let block_id: BlockId = block_number.into();

    let timestamp = provider.get_block(block_id, BlockTransactionsKind::Hashes)
        .await
        .map_err(|e| format!("RPC Error: {:?}", e))?
        .ok_or("No block")?
        .header.timestamp;
    
    Ok((res.clone(), res.clone()))
}

async fn send_to_cache(redis: &mut Connection, key: &str, value: &str) -> Result<bool, RedisError> {
    match redis.set::<&str, &str, ()>(key, value) {
        Ok(_) => {
            return Ok(true)
        }
        Err(e) => {
            return Err(e);
        }
    }
}
#[tokio::main]
async fn main(){

    dotenv::dotenv().ok();

    let rpc_url_http = std::env::var("RPC_URL_HTTP").expect("RPC_URL_HTTP must be set");
    let topic = std::env::var("TOPIC").expect("TOPIC must be set");

    // let topic = "test-topic";
    let consumer = ClientConfig::new()
        .set("group.id", "test-group")
        .set("bootstrap.servers", "localhost:9092")
        .create::<StreamConsumer>()
        .expect("Consumer creation failed");
    consumer.subscribe(&[topic.as_str()]).expect("Can't subscribe to specified topic");

    let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> = 
            ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
    let client = redis::Client::open("redis://localhost:6379").expect("Failed to connect to Redis");
    let mut con = client.get_connection().expect("Failed to get connection");

    loop {
        let message = consumer.recv().await.expect("Consumer failure");
        let message = message.detach();
        let message = message.payload();
        let mut hash = String::new();
        match message {
            None => {
                println!("No message");
                continue;
            }
            Some(payload) => {
                let content = std::str::from_utf8(payload).expect("Invalid UTF-8");
                hash = content.to_string();
            }
        }
        let mut key = String::new();
        let mut value = String::new();
        match decode_tx(&http_provider, &hash).await {
            Err(e) => {
                println!("Ignoring {}, Error: {}", hash, e);
                continue;
            }
            Ok((k,v)) => {
                println!("Decoded tx, {}, {}", key, value);
                key = k;
                value = v;
            }
        }
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