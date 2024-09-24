use alloy::signers::k256::elliptic_curve::rand_core::block;
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};

use futures::executor::block_on;
use std::{
    future::Future,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};

pub struct Feline {
    pub name: String,
}

impl Future for Feline {
    type Output = String;

    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        println!("polling {}", self.name);
        std::task::Poll::Ready(format!("{} is a feline", self.name))
    }
}
async fn meow(){
    println!("meow");
    Feline {
        name: "Mittens".to_string(),
    }.await;
}

// fn woof() -> impl Future<Output = i32> {
async fn woof() -> i32 {
    println!("woof");
    // block_on(meow());
    return 1;
}


async fn async_main(){
    futures::join!(woof(), meow());
}

fn main() {
    let cody = Feline {
        name: "Cody".to_string(),
    };
    block_on(cody);
    let x = async_main();
    block_on(x);
    // let mut cat = Cat(vec![Box::pin(feline)]);
    // cat.block_on_all();
}