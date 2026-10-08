use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock, atomic::{AtomicUsize, Ordering::Relaxed},}, thread};

use mio::{Events, Interest, Poll, Registry, event::Source};

use crate::runtime::{Waker};

// allow leaf futures to register interest and pass there wakers. Once an event has occured, call the corresponding waker's wake.
type Wakers = Arc<Mutex<HashMap<usize, Waker>>>; //since this data will be written by futures on different threads, mutex therefore
//again a static variable, so that it can be accessed from anywhere
static CURRENT_REACTOR: OnceLock<Reactor> = OnceLock::new();

struct Reactor {
    //these are the wakers passed to us by the leaf futures
    wakers: Wakers,
    //simple maintain an internal registery which we will block on once we are free, this does not mean the futures are not progressing, never.
    registry: Registry,
    //unique id that leaf futures will use while sending tasks to register interest in
    next_id: AtomicUsize
}

//ofcourse the reactor should expose functions that allow futures to register interest in some I/O

impl Reactor {
    pub fn register(&self, source: &mut impl Source, interest: Interest, id: usize, waker: &Waker) {
        //add to registry
        self.registry.register(source, mio::Token(id), interest);
        //now register waker
        self.wakers.lock().unwrap().insert(id, waker.clone());
    }

    pub fn next_id(&self) -> usize {
        //give out next id, by incrementing. atomically safe.
        self.next_id.fetch_add(1, Relaxed)
    }

}

    fn event_loop(mut poll: Poll, wakers: Wakers) {
        //this is responsible for wait on mulitple events. If an event occurs, call wake on associated waker
        let mut events = Events::with_capacity(100);
        loop {
            poll.poll(&mut events, None);
            //reaches here once some event happens
            for event in events.iter() {
                let id = event.token().0;
                //find the corresponsing worker and call wake on it
                if let Some(waker) = wakers.lock().unwrap().get(&id) {
                    waker.wake();
                }
            }
        }
    }

    //what do we need to start the reactor ?
    pub fn start() {
        //spawn a seperate thread that runs independently, and its sole job is to REACT
        //on that seperate thread we will run our event loop, which requires poll and wakers
        let wakers: Wakers = Arc::new(Mutex::new(HashMap::new()));
        let poll = Poll::new().unwrap();
        //almost there, the event loop will look on the static variable holding current executor, so let's place it there
        CURRENT_REACTOR.set(Reactor { wakers: wakers.clone(), registry : poll.registry().try_clone().unwrap(), next_id: AtomicUsize::new(1) });
        //static variable has the value, now kick off the event loop
        //move because we dont need anymore
        thread::spawn(move || {
            event_loop(poll, wakers);
        });
    }
