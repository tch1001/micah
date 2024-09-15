use core::time;
use std::{
    collections::HashMap,
    ffi::c_char,
    hash::Hash,
    io::{BufRead, Write},
};

use alloy::{
    eips::BlockId,
    primitives::TxHash,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::{Block, BlockTransactionsKind},
};
use eyre::Result;
use futures_util::StreamExt;

fn read_file(existing_hashes: &mut Vec<String>, file: &std::fs::File) {
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let hash = line.unwrap();
        existing_hashes.push(hash);
    }
}

async fn erc20_token_transfer_events() {
    //     https://api.etherscan.io/api
    //    ?module=account
    //    &action=tokentx
    //    &contractaddress=0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2
    //    &address=0x4e83362442b8d1bec281594cea3050c8eb01311c
    //    &page=1
    //    &offset=100
    //    &startblock=0
    //    &endblock=27025780
    //    &sort=asc
    //    &apikey=YourApiKeyToken
    let url = "https://api.etherscan.io/api";
    let module = "account";
    let action = "tokentx";
    let contractaddress = "0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2";
    let address = "0x4e83362442b8d1bec281594cea3050c8eb01311c";
    let page = "1";
    let offset = "100";
    let startblock = "0";
    let endblock = "27025780";
    let sort = "asc";
    let apikey = std::env::var("ETHERSCAN_API_KEY").unwrap();
    let url = format!(
        "{}?module={}&action={}&contractaddress={}&address={}&page={}&offset={}&startblock={}&endblock={}&sort={}&apikey={}",
        url, module, action, contractaddress, address, page, offset, startblock, endblock, sort, apikey
    );
    println!("url = {}", url);
    let resp = reqwest::get(&url).await.unwrap();
    // println!("{:?}", resp.text().await.unwrap());
    // {
    //   "status": "1",
    //   "message": "OK",
    //   "result": [
    //     {
    //       "blockNumber": "4730207",
    //       "timeStamp": "1513240363",
    //       "hash": "0xe8c208398bd5ae8e4c237658580db56a2a94dfa0ca382c99b776fa6e7d31d5b4",
    // get the hash from above from resp.json()
    let resp = resp.json::<serde_json::Value>().await.unwrap();
    let mut file = std::fs::File::options()
        .read(true)
        .write(true)
        .open("erc20_token_transfer_events.txt")
        .unwrap();
    let mut existing_hashes = vec![];
    read_file(&mut existing_hashes, &mut file);
    for entry in resp["result"].as_array().unwrap() {
        let hash = entry["hash"].as_str().unwrap();
        if existing_hashes.contains(&hash.to_string()) {
            continue;
        }
        println!("hash = {} appended", hash);
        // append to file
        writeln!(file, "{}", hash).unwrap();
    }
}

// return timestamp and gas fee
async fn decode_tx(
    provider: &RootProvider<alloy::transports::http::Http<reqwest::Client>>,
    hash: &str,
) -> Option<(u64, u128)> {
    let tx_hash: TxHash = hash.parse().unwrap();
    println!("tx_hash = {:?}", tx_hash);
    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();
    let tx = provider
        .get_transaction_by_hash(tx_hash)
        .await
        .unwrap()
        .unwrap();
    // println!("tx = {:?}", tx);
    // println!("receipt = {:?}", receipt);
    let block_number = receipt.block_number.unwrap();
    let gas_price = tx.gas_price.unwrap();
    let gas_used = receipt.gas_used;
    let gas_fee = gas_price * gas_used;
    let block_id: BlockId = block_number.into();
    let timestamp = provider
        .get_block(block_id, BlockTransactionsKind::Full)
        .await
        .unwrap()
        .unwrap();
    let timestamp = timestamp.header.timestamp;
    return Some((timestamp, gas_fee));
}

async fn binance_eth_usdt_price(time_ms: u64) -> Option<(u64, f64)> {
    // get the price over 10s
    if false {
        for i in 0..10 {
            let url = "https://api.binance.com/api/v3/ticker/price?symbol=ETHUSDT";
            let resp = reqwest::get(url).await.unwrap();
            let resp = resp.json::<serde_json::Value>().await.unwrap();
            // println!("resp = {:?}", resp["price"]);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    if false {
        let mut historical_price: HashMap<u64, f64> = HashMap::new();
        for i in 0..1 {
            let url = "https://api.binance.com/api/v3/historicalTrades?symbol=ETHUSDT";
            let resp = reqwest::get(url).await.unwrap();
            let resp = resp.json::<serde_json::Value>().await.unwrap();
            // println!("resp = {:?}", resp.as_array().unwrap()[0]);
            for entry in resp.as_array().unwrap() {
                let data: &serde_json::Map<String, serde_json::Value> = entry.as_object().unwrap();
                // println!("data = {:?}", data["time"].as_number().unwrap().);
                let price: f64 = data["price"].as_str().unwrap().parse::<f64>().unwrap();
                let time: u64 = data["time"].as_number().unwrap().as_u64().unwrap();
                println!("price = {}, time = {}", price, time);
                historical_price.insert(time, price);
            }
        }
        println!("historical_price = {:?}", historical_price);
    }
    if true {
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
                    let data: &serde_json::Map<String, serde_json::Value> =
                        entry.as_object().unwrap();
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
                return Some((closest_time, returned_price));
            }
        }
    }
    return None;
}

#[no_mangle]
pub extern "C" fn woof() {
    println!("woof from rust");
}
#[no_mangle]
pub extern "C" fn process_tx(hash: *const c_char) -> f64 {
    let hash = unsafe { std::ffi::CStr::from_ptr(hash) };
    let hash = hash.to_str().unwrap();
    println!("hash = {}", hash);
    dotenv::dotenv().ok();
    let rpc_url_ws: String = std::env::var("RPC_URL_WS").unwrap();
    let rpc_url_http: String = std::env::var("RPC_URL_HTTP").unwrap();
    let ws: WsConnect = WsConnect::new(rpc_url_ws);
    // let provider: RootProvider<PubSubFrontend> = ProviderBuilder::new().on_ws(ws).await.unwrap();
    let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> =
        ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gas_fee_usdt = runtime.block_on(async {
        let (timestamp, gas_fee) = decode_tx(&http_provider, hash).await.unwrap();
        let (closest_timestamp, eth_usdt_price) =
            binance_eth_usdt_price(timestamp * 1000).await.unwrap();
        println!("eth_usdt_price = {:?}", eth_usdt_price);
        let gas_fee_usdt = gas_fee as f64 * eth_usdt_price / 1e18;
        println!("gas_fee_usdt = {:?}", gas_fee_usdt);
        gas_fee_usdt
    });
    println!("return to c: gas_fee_usdt = {:?}", gas_fee_usdt);
    gas_fee_usdt
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let rpc_url_ws: String = std::env::var("RPC_URL_WS").unwrap();
    let rpc_url_http: String = std::env::var("RPC_URL_HTTP").unwrap();
    println!("RPC_URL_WS = {}", rpc_url_ws);

    // erc20_token_transfer_events().await;
    // return Ok(());

    let ws: WsConnect = WsConnect::new(rpc_url_ws);
    // let provider = ProviderBuilder::new().on_ws(ws).await?;
    let provider: RootProvider<PubSubFrontend> = ProviderBuilder::new().on_ws(ws).await?;
    let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> =
        ProviderBuilder::new().on_http(rpc_url_http.parse()?);
    let (timestamp, gas_fee) = decode_tx(
        &http_provider,
        "0xe8c208398bd5ae8e4c237658580db56a2a94dfa0ca382c99b776fa6e7d31d5b4",
    )
    .await
    .unwrap();
    let (closest_timestamp, eth_usdt_price) =
        binance_eth_usdt_price(timestamp * 1000).await.unwrap();
    println!("eth_usdt_price = {:?}", eth_usdt_price);
    let gas_fee_usdt = gas_fee as f64 * eth_usdt_price / 1e18;
    println!("gas_fee_usdt = {:?}", gas_fee_usdt);
    return Ok(());
    let sub: alloy::pubsub::Subscription<Block> = provider.subscribe_blocks().await?;
    let mut stream: futures_util::stream::Take<alloy::pubsub::SubscriptionStream<Block>> =
        sub.into_stream().take(4);
    let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        while let Some(block) = stream.next().await {
            println!("Block: {:?}", block);
        }
    });
    handle.await?;
    Ok(())
}
