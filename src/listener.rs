use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use futures::StreamExt;
use rdkafka::producer::FutureProducer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let topic = std::env::var("TOPIC").expect("TOPIC must be set");
    let kafka_broker = std::env::var("KAFKA_BROKER").expect("KAFKA_BROKER must be set");
    let producer: FutureProducer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", kafka_broker.as_str())
        .create()
        .expect("Producer creation error");

    let rpc_url_ws = std::env::var("RPC_URL_WS").expect("RPC_URL_WS must be set");
    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(rpc_url_ws))
        .await?;

    let sub = provider.subscribe_pending_transactions().await?;
    // let mut stream = sub.into_stream().take_while()
    // while let Some(tx) = stream.next().await {
    //     let tx = tx?;
    //     let tx_hash = tx.hash;
    //     println!("Received tx: {}", tx_hash);
    // }

    Ok(())
}
