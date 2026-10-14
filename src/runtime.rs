use std::{thread::Builder};

pub use executor::{Executor};
pub use reactor::{reactor};
use std::{future::Future};

mod executor;
mod reactor;
pub fn init() -> Executor {
    reactor::start();
    Executor::new()
}

pub fn join_all<T: Future<Output = ()> + Send + 'static>(mut futures: Vec<T>) {
    //why does the output define unit ? why not take a generic ?
    //I did try to make everything generic up to the executor, but turns out rust does not allow static generics (and executor has to be static at the top)
    //other runtimes like tokio do this by providing a join_handle
                    
    //in order to kick off the reactor:
    let mut executor = init();
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