use alloy::{providers::{Provider, ProviderBuilder, RootProvider, WsConnect}, pubsub::PubSubFrontend, rpc::types::Block};
use eyre::Result;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let url: String = std::env::var("RPC_URL_WS").unwrap();
    println!("RPC_URL_WS = {}", url);
    let ws = WsConnect::new(url);
    // let provider = ProviderBuilder::new().on_ws(ws).await?;
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let sub = provider.subscribe_blocks().await?;
    let mut stream = sub.into_stream().take(4);
    let handle = tokio::spawn(async move {
        while let Some(block) = stream.next().await {
            println!("Block: {:?}", block);
        }
    });
    handle.await?;
    Ok(())
}

// Example of using the WS provider to subscribe to new blocks.

// use alloy::providers::{Proider, ProviderBuilder, WsConnect};
// use eyre::Result;
// use futures_util::StreamExt;

// #[tokio::main]
// async fn main() -> Result<()> {
//     // Set up the WS transport which is consumed by the RPC client.
//     dotenv::dotenv().ok();
//     let url: String = std::env::var("RPC_URL_WS").unwrap();
//     // let rpc_url = "wss://eth-mainnet.g.alchemy.com/v2/your-api-key";

//     // Create the provider.
//     let ws = WsConnect::new(url);
//     let provider = ProviderBuilder::new().on_ws(ws).await?;

//     // // Subscribe to new blocks.
//     // let sub = provider.subscribe_blocks().await?;

//     // // Wait and take the next 4 blocks.
//     // let mut stream = sub.into_stream().take(4);

//     // println!("Awaiting blocks...");

//     // // Take the stream and print the block number upon receiving a new block.
//     // let handle = tokio::spawn(async move {
//     //     while let Some(block) = stream.next().await {
//     //         println!("Latest block number: {}", block.header.number);
//     //     }
//     // });

//     // handle.await?;

//     Ok(())
// }
