// use alloy::signers::k256::elliptic_curve::rand_core::block;
// use futures::{
//     future::{BoxFuture, FutureExt},
//     task::{waker_ref, ArcWake},
// };

// use futures::executor::block_on;
// use std::{
//     future::Future,
//     sync::mpsc::{sync_channel, Receiver, SyncSender},
//     sync::{Arc, Mutex},
//     task::Context,
//     time::Duration,
// };

// pub struct Feline {
//     pub name: String,
// }

// impl Future for Feline {
//     type Output = String;

//     fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
//         println!("polling {}", self.name);
//         // std::task::Poll::Ready(format!("{} is a feline", self.name))
//         // _cx.waker().wake_by_ref();
//         let waker = _cx.waker();
//         std::task::Poll::Pending
//     }
// }
// async fn meow(){
//     println!("meow");
//     Feline {
//         name: "Mittens".to_string(),
//     }.await;
// }

// // fn woof() -> impl Future<Output = i32> {
// async fn woof() -> i32 {
//     println!("woof");
//     // block_on(meow());
//     return 1;
// }

// async fn async_main(){
//     futures::join!(woof(), meow());
// }

// fn main() {
//     let cody = Feline {
//         name: "Cody".to_string(),
//     };
//     block_on(cody);
//     let x = async_main();
//     block_on(x);
//     // let mut cat = Cat(vec![Box::pin(feline)]);
//     // cat.block_on_all();
// }

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio;

async fn adder(hm: Arc<Mutex<HashMap<String, i32>>>, ctr: i32) {
    let ctr_str: String = ctr.to_string();
    let mut hm = hm.lock().unwrap();
    hm.insert(ctr_str, ctr);
}
#[tokio::main]
async fn main() {
    let hm: Mutex<HashMap<String, i32>> = Mutex::new(HashMap::new());
    let hm_arc = Arc::new(hm);
    for i in 0..1000 {
        let hm_arc_clone = Arc::clone(&hm_arc);
        tokio::spawn(async move {
            adder(hm_arc_clone.clone(), i).await;
            // size of hashmap
            println!("{:?}", hm_arc_clone.lock().unwrap().len());
        });
    }
}
