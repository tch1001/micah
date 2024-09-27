use alloy::providers::{ProviderBuilder, RootProvider};
use rdkafka::{consumer::{Consumer, StreamConsumer}, ClientConfig, Message};
async fn decode_tx(provider, hash) {

}

async fn send_to_cache(redis, key, value) {

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

    let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> = ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());

    loop {
        let message = consumer.recv().await.expect("Consumer failure");
        let message = message.detach();
        // println!("Message: {:?}", message);
        let message = message.payload();
        match message {
            None => println!("No message"),
            Some(payload) => {
                let content = std::str::from_utf8(payload).expect("Invalid UTF-8");
                // println!("Content: {}", content);
                let (key, value) = decode_tx(http_provider_clone, hash).await;
                match send_to_cache(redis, key, value).await {
                    Ok(_) => println!("Sent to cache"),
                    Err(e) => println!("Error: {}", e)
                }
            }
        }
    }
}