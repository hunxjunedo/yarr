use std::{vec};

use crate::{future::PollState, http::Http, runtime::{Waker, join_all}};

mod future;
mod runtime;
mod http;

fn main() {
    let mut futures = vec![];
    for i in 1..13 {
        let fut = request(i);
        futures.push(fut);
    }
    join_all(futures);
    
}

// coroutine fn async_main() {
//     let mut buffer = String::from("\nBUFFER:\n----\n");
// let writer = &mut buffer;
// println!("Program starting");
// let txt = http::Http::get("/600/HelloAsyncAwait").wait;
// writeln!(writer, "{txt}").unwrap();
// let txt = http::Http::get("/400/HelloAsyncAwait").wait;
// writeln!(writer, "{txt}").unwrap();
// println!("{}", buffer);
// }






// =================================
// We rewrite this:
// =================================
    
// coroutine fn request(i: usize) {
//     let path = format!("/{}/HelloWorld{i}", i * 1000);
//     let txt = Http::get(&path).wait;
//     let txt = txt.lines().last().unwrap_or_default();
//     println!(«{txt}»)

// }

// =================================
// Into this:
// =================================

fn request(i: usize) -> impl future::Future<Output=String> {
    Coroutine0::new(i)
}
        
enum State0 {
    Start(usize),
    Wait1(Box<dyn future::Future<Output = String>>),
    Resolved,
}

struct Coroutine0 {
    state: State0,
}

impl Coroutine0 {
    fn new(i: usize) -> Self {
        Self { state: State0::Start(i) }
    }
}

unsafe impl Send for Coroutine0 {}


impl future::Future for Coroutine0 {
    type Output = String;

    fn poll(&mut self, waker: &Waker) -> PollState<Self::Output> {
        loop {
        match self.state {
                State0::Start(i) => {
                    // ---- Code you actually wrote ----
                    let path = format!("/{}/HelloWorld{i}", i * 1000);

                    // ---------------------------------
                    let fut1 = Box::new( Http::get(path));
                    self.state = State0::Wait1(fut1);
                }

                State0::Wait1(ref mut f1) => {
                    match f1.poll(waker) {
                        PollState::Ready(txt) => {
                            // ---- Code you actually wrote ----
                            let txt = txt.lines().last().unwrap_or_default();
    println!("«{txt}»");

                            // ---------------------------------
                            self.state = State0::Resolved;
                            break PollState::Ready(String::new());
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Resolved => panic!("Polled a resolved future")
            }
        }
    }
}
