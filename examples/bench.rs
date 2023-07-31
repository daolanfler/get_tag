use std::time::{Duration, Instant};
use tokio::runtime::Builder;

fn main() {
    let runtime = Builder::new_current_thread().build().unwrap();
    runtime.block_on(async {
        let now = Instant::now();
        for _ in 0..100_000 {
            v1().await;
        }
        println!("tokio::spawn = {:?}", now.elapsed());

        let now = Instant::now();
        for _ in 0..100_000 {
            v2().await;
        }
        println!("tokio::join! = {:?}", now.elapsed());
    });
}

async fn v1() {
    let t1 = tokio::spawn(do_nothing());
    let t2 = tokio::spawn(do_nothing());
    t1.await.unwrap();
    t2.await.unwrap();
}

async fn v2() {
    let t1 = do_nothing();
    let t2 = do_nothing();
    tokio::join!(t1, t2);
}

async fn do_nothing() {}
