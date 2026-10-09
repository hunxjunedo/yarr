use std::{future::Future, thread::Builder, vec};

use crate::{future::PollState, http::Http, runtime::{Executor, Waker}};

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





fn join_all<T: future::Future<Output = String> + Send + 'static>(mut futures: Vec<T>) {
    //in order to kick off the reactor:
    let mut executor = runtime::init();
    //now a reactor has started, it is listening to all the events. 
    // executor polls a future with a waker. And the future uses the global reactor context to register interest.
    let mut handles = vec![];
    //this is for progressing tasks concurrently and in parallel, if we had a single executor on a single thread, it would have definetly have been concurrent since all progressing together, but only one at a time.

    //note: the design is such that creating a runtime gives you a reactor and an executor. ofcourse you can spawn any number of executors, but what do you do with one extra you already got  ?
    //solution: just use it, like this:
    let first_future = futures.remove(0);
    executor.block_on(first_future);
for (i, future) in (1..).zip(futures.into_iter()) {
    // i starts from 1 automatically
    let name = format!("exec-{i}");
    let h = Builder::new().name(name).spawn( move || {
        let mut executor = Executor::new();
        executor.block_on(future);
    }).unwrap();
    handles.push(h);
}
    //each thread is blocking on a future, now what ?
    //do not return until all threads end, aka all executors say no more tasks to perform, aka all top-level futures are resolved
    handles.into_iter().for_each(|h| h.join().unwrap());

    //question: why dont we need to block on reactor, aka block on the thread thats running event loop ?
    //because we are already blocking on the executors to end. An executor will only end once all its tasks are resolved, 
    //and if that happens, you don't need a reactor anymore. Remember that a future, before resolving (returning Ready) dergeisters all the interests it registered,
    //so you should not expect a notification either way.
}

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
