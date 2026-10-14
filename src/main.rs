mod http;
mod runtime;
use http::Http;

use crate::runtime::join_all;
fn main() {
    let mut executor = runtime::init();
    executor.block_on(async_main());
}
async fn async_main() {
    println!("Program starting");
    let txt = Http::get("/600/HelloAsyncAwait".to_string()).await;
    println!("{txt}");
    let txt = Http::get("/400/HelloAsyncAwait".to_string()).await;
    println!("{txt}");
}
type T = dyn Future<Output = ()> + Send;
async fn join_tester() {
    //the goal is to check if join_all function works as expected
    let futures: Vec<std::pin::Pin<Box<T>>> = vec![
        Box::pin(async {
            let output = Http::get("/600/HelloAsyncAwait".to_string()).await;
            println!("output recieved from a future: {}", output)
        }),
        Box::pin(async {
            let output = Http::get("/600/HelloAsyncAwait".to_string()).await;
            println!("output recieved from a future: {}", output)
        }),
        Box::pin(async {
            let output = Http::get("/600/HelloAsyncAwait".to_string()).await;
            println!("output recieved from a future: {}", output)
        }),
        Box::pin(async {
            let output = Http::get("/600/HelloAsyncAwait".to_string()).await;
            println!("output recieved from a future: {}", output)
        }),
    ];
    return join_all(futures);
}
