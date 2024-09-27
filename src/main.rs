use core::time;
use std::{
    collections::HashMap, ffi::c_char, hash::Hash, io::{BufRead, BufReader, Write}, net::{TcpListener, TcpStream}, sync::Arc, thread, time::Duration
};
use rdkafka::producer::{FutureProducer, FutureRecord};

use alloy::{
    eips::BlockId,
    primitives::TxHash,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::{Block, BlockTransactionsKind, Transaction},
};
use eyre::Result;
use futures_util::StreamExt;
use tokio::{runtime, sync::futures, task};

use tokio::sync::Mutex;

use std::sync::Mutex as StdMutex;


fn read_file(existing_hashes: &mut Vec<String>, file: &std::fs::File) {
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let hash = line.unwrap();
        existing_hashes.push(hash);
    }
}

static UNISWAP_V3 : &str = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640";
async fn erc20_token_transfer_events(startblock: u64, endblock: u64) -> Vec<String> {
    //    https://api.etherscan.io/api
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
    // https://api.etherscan.io/api?module=account&action=tokentx&address=0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640&page=1&offset=100&sort=desc&apikey=API
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
    let apikey = std::env::var("ETHERSCAN_API_KEY").unwrap();
    let url = format!(
        "{}?module={}&action={}&address={}&page={}&offset={}&sort={}&apikey={}",
        url, module, action, address, page, offset, sort, apikey
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
    // let mut file = std::fs::File::options()
    //     .read(true)
    //     .write(true)
    //     .open("erc20_token_transfer_events.txt")
    //     .unwrap();
    let mut existing_hashes: Vec<String> = vec![];
    // read_file(&mut existing_hashes, &mut file);
    for entry in resp["result"].as_array().unwrap() {
        let hash = entry["hash"].as_str().unwrap();
        existing_hashes.push(hash.to_string());
        // if existing_hashes.contains(&hash.to_string()) {
        //     continue;
        // }
        // println!("hash = {} appended", hash);
        // append to file
        // writeln!(file, "{}", hash).unwrap();
    }
    existing_hashes
}

// return timestamp and gas fee
// cache it with a hashmap

async fn decode_tx(
    provider: Arc<Mutex<RootProvider<alloy::transports::http::Http<reqwest::Client>>>>,
    hash: &str,
    cache: Arc<Mutex<HashMap<String, (u64, u128)>>>,
) -> Option<(u64, u128)> {
    let tx_hash: TxHash = hash.parse().unwrap();
    println!("tx_hash = {:?}", tx_hash);
    let mut local_cache = cache.lock().await;
    println!("got lock");
    if local_cache.contains_key(hash) {
        return Some(local_cache[hash]);
    }
    let provider = provider.lock().await;
    println!("got lock 2");
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
    // let tmp = cache.lock().await;
    // tmp.insert(k, v);
    local_cache.insert(hash.to_string(), (timestamp, gas_fee));
    return Some((timestamp, gas_fee));
}

async fn binance_eth_usdt_price(
    time_ms: u64,
    eth_usdt_price_cache: Arc<Mutex<HashMap<u64, (u64, f64)>>>,
) -> Option<(u64, f64)> {
    let mut local_cache = eth_usdt_price_cache.lock().await;
    if local_cache.contains_key(&time_ms) {
        return Some(local_cache[&time_ms]);
    }
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
            local_cache.insert(time_ms, (closest_time, returned_price));
            return Some((closest_time, returned_price));
        }
    }
    return None;
}

async fn process_tx_rust(
    hash: &str,
    http_provider: Arc<Mutex<RootProvider<alloy::transports::http::Http<reqwest::Client>>>>,
    cache: Arc<tokio::sync::Mutex<HashMap<String, (u64, u128)>>>,
    eth_usdt_price_cache: Arc<tokio::sync::Mutex<HashMap<u64, (u64, f64)>>>,
) -> f64 {
    let (timestamp, gas_fee) = decode_tx(http_provider, hash, cache).await.unwrap();
    let (_, eth_usdt_price) = binance_eth_usdt_price(timestamp * 1000, eth_usdt_price_cache)
        .await
        .unwrap();
    let gas_fee_usdt = gas_fee as f64 * eth_usdt_price / 1e18;
    println!("gas_fee_usdt = {:?}", gas_fee_usdt);
    return gas_fee_usdt;
}

async fn handle_connection(
    mut stream: TcpStream,
    http_provider: Arc<Mutex<RootProvider<alloy::transports::http::Http<reqwest::Client>>>>,
    cache: Arc<Mutex<HashMap<String, (u64, u128)>>>,
    eth_usdt_price_cache: Arc<Mutex<HashMap<u64, (u64, f64)>>>,
) {
    let buf_reader = BufReader::new(&mut stream);
    // let http_request = buf_reader.lines().map(|line| line.unwrap()).take_while(|line| !line.is_empty()).collect::<Vec<String>>();
    let request_line = buf_reader.lines().next().unwrap().unwrap();
    // Extract the path from GET /path HTTP/1.1
    let path = request_line.split_whitespace().nth(1).unwrap();
    let mut path = path.to_string();
    path += "?"; // hacky way to get split by ? to work
                 // split by question mark before and after
    let action_and_params = path.split("?");
    // println!("action_and_params = {:?}", action_and_params.clone().collect::<Vec<&str>>());
    let action_and_params = action_and_params.collect::<Vec<&str>>();
    let action = action_and_params[0];
    if action == "/tx" {
        let params = action_and_params[1];
        // println!("params = {:?}", params);
        assert!(params.starts_with("hash="));
        let hash = params.split("=").nth(1).unwrap();
        println!("hash = {:?}", hash);
        assert!(hash.len() == 66);
        let gas_fee_usdt = process_tx_rust(hash, http_provider, cache, eth_usdt_price_cache).await;
        // println!("gas_fee_usdt = {:?}", gas_fee_usdt);
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", gas_fee_usdt);
        stream.write_all(response.as_bytes()).unwrap();
    } else if action == "/clear_cache" {
        cache.lock().await.clear();
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", "cache cleared");
        stream.write_all(response.as_bytes()).unwrap();
    } else if action == "/sleep" {
        thread::sleep(Duration::from_secs(10)); // simulate a long running task
                                                // multithreaded web server should be able to handle other requests concurrently
    } else if action == "/batch" {
        let params_str = action_and_params[1];
        let params_vec = params_str.split("&").collect::<Vec<&str>>();
        let start_block_str = params_vec
            .iter()
            .find(|&x| x.starts_with("start_block="))
            .unwrap();
        let start_block_int = start_block_str
            .split("=")
            .nth(1)
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let end_block_str = params_vec
            .iter()
            .find(|&x| x.starts_with("end_block="))
            .unwrap();
        let end_block_int = end_block_str
            .split("=")
            .nth(1)
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let tx_list = erc20_token_transfer_events(start_block_int, end_block_int).await;
        println!("Found {} txs", tx_list.len());
        let mut processed_tx_list = vec![];
        for tx in tx_list {
            let gas_fee_usdt = process_tx_rust(&tx, http_provider.clone(), cache.clone(), eth_usdt_price_cache.clone()).await;
            processed_tx_list.push((tx,gas_fee_usdt));
        }
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{:?}", processed_tx_list);
        stream.write_all(response.as_bytes()).unwrap();
    } else {
        let response = format!("HTTP/1.1 404 NOT FOUND\r\n\r\n{}", "404 NOT FOUND");
        stream.write_all(response.as_bytes()).unwrap();
    }
    // println!("path = {}", path);
    // println!("http_request = {:?}", http_request);
}

async fn new_blocks_listener(http_provider: Arc<Mutex<RootProvider<alloy::transports::http::Http<reqwest::Client>>>>) -> Result<()> {
    let rpc_url_ws: String = std::env::var("RPC_URL_WS").unwrap();
    println!("RPC_URL_WS = {}", rpc_url_ws);

    // erc20_token_transfer_events().await;
    // return Ok(());

    let ws: WsConnect = WsConnect::new(rpc_url_ws);
    // let provider = ProviderBuilder::new().on_ws(ws).await?;
    let provider: RootProvider<PubSubFrontend> = ProviderBuilder::new().on_ws(ws).await?;
    let rpc_url_http: String = std::env::var("RPC_URL_HTTP").unwrap();
    // let http_provider: RootProvider<alloy::transports::http::Http<reqwest::Client>> =
    //     ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
    let mut cache: HashMap<String, (u64, u128)> = HashMap::new();
    let cache_arc = Arc::new(tokio::sync::Mutex::new(cache));
    let mut eth_usdt_price_cache: HashMap<u64, (u64, f64)> = HashMap::new();
    let eth_usdt_arc = Arc::new(tokio::sync::Mutex::new(eth_usdt_price_cache));
    let sub: alloy::pubsub::Subscription<alloy::primitives::FixedBytes<32>> = provider.subscribe_pending_transactions().await?;
    let mut stream =
        sub.into_stream().take_while(|_| futures_util::future::ready(true));
    let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        while let Some(tx_hash) = stream.next().await {
            // if it's calling the uniswap address
            let tx_response = provider.get_transaction_by_hash(tx_hash).await;
            if tx_response.is_err() {
                continue;
            }
            let tx = tx_response.unwrap();
            if tx.is_none() {
                continue;
            }
            let tx = tx.unwrap();
            // println!("Tx Hash: {:?}", tx_hash);
            if tx.block_number.is_none() {
                continue;
            }
            if tx.to.is_none() {
                continue;
            }
            let tx_to_address = tx.to.unwrap();
            let tx_to_str = tx_to_address.to_string();
            if tx_to_str == UNISWAP_V3 {
                let tx_hash_str = tx_hash.to_string();
                let tx_hash_str = tx_hash_str.as_str();
                process_tx_rust(tx_hash_str, http_provider.clone(), cache_arc.clone(), eth_usdt_arc.clone()).await;
            }
            println!("Tx Hash: {:?}", tx_hash);
        }
    });
    handle.await?;
    Ok(())
}

// #[derive(Copy, Clone)]
struct Point {
    x: i32,
    y: i32
}
enum Nat {
    Zero,
    Succ(Box<Nat>)
}

struct MyBox<T> {
    data: T
}

impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        return MyBox {data : x};
    }
}

struct Wolf{}
impl Wolf{}

trait MyClone<T> {
    fn clone(&self) -> T ;
}
impl MyClone<Point> for Point {
    fn clone(&self) -> Point {
        return Point {x : self.x, y : self.y};
    }
}
fn id(p : Point) -> Point {
    return p;
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    // erc20_token_transfer_events(0, 100).await;
    // let producer : FutureProducer = rdkafka::config::ClientConfig::new()
    //     .set("bootstrap.servers", "localhost:9092")
    //     .set("message.timeout.ms", "5000")
    //     .create()
    //     .expect("Producer creation error");
    // let topic = "test-topic";
    // let value = "Hello, world!";
    // let meow = producer
    //         .send(
    //             FutureRecord::to(&topic)
    //                 .payload(value.as_bytes())
    //                 .key("alice"), Duration::from_secs(1)
    //             );
    // let x = meow.await;
    // println!("x = {:?}", x);
    let rpc_url_http: String = std::env::var("RPC_URL_HTTP").unwrap();
    let http_prov = ProviderBuilder::new().on_http(rpc_url_http.parse().unwrap());
    let http_provider: Arc<Mutex<RootProvider<alloy::transports::http::Http<reqwest::Client>>>> = Arc::new(Mutex::new(http_prov));
    let cache: Arc<tokio::sync::Mutex<HashMap<String, (u64, u128)>>> = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let eth_usdt_price_cache: Arc<tokio::sync::Mutex<HashMap<u64, (u64, f64)>>> = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let hpc = http_provider.clone();
    task::spawn(async {
        let _ = new_blocks_listener(hpc).await;
    });
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let hpc = http_provider.clone();
        let cache_clone = cache.clone();
        let eupcc = eth_usdt_price_cache.clone();
        task::spawn(async { // doesn't work because of shared cache
            handle_connection(
                stream,
                hpc,
                cache_clone,
                eupcc,
            )
            .await;
        });
    }
    

    Ok(())
}
