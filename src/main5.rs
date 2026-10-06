mod future;
mod http;
mod runtime;
use std::future::ready;
use future::{Future, PollState};
use runtime::Runtime;
fn main() {
    let future = async_main();
    let mut runtime = Runtime::new();
    runtime.block_on(future);
}
coroutine fn async_main() {
    println!("Program starting");
    let txt = http::Http::get("/600/HelloAsyncAwait").wait;
    println!("{txt}");
    let txt = http::Http::get("/400/HelloAsyncAwait").wait;
    println!("{txt}");
}


struct Ready {
    text: String
}
impl Ready {
     fn new(txt: String) -> Self {
        Self { text:  txt}
    }
}
impl Future for Ready {
    type Output = String;

    fn poll(&mut self) -> PollState<Self::Output> {
        PollState::Ready(String::from("Sup bitch"))
    }
}