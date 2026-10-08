use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock, atomic::{AtomicUsize, Ordering::Relaxed},}, thread};

use mio::{Events, Interest, Poll, Registry, event::Source};

use crate::runtime::{Waker};

// allow leaf futures to register interest and pass there wakers. Once an event has occured, call the corresponding waker's wake.
type Wakers = Arc<Mutex<HashMap<usize, Waker>>>; //since this data will be written by futures on different threads, mutex therefore
//again a static variable, so that it can be accessed from anywhere
static CURRENT_REACTOR: OnceLock<Reactor> = OnceLock::new();

pub struct Reactor {
    //these are the wakers passed to us by the leaf futures
    wakers: Wakers,
    //simple maintain an internal registery which we will block on once we are free, this does not mean the futures are not progressing, never.
    registry: Registry,
    //unique id that leaf futures will use while sending tasks to register interest in
    next_id: AtomicUsize
}

pub fn reactor() -> &'static Reactor {
    CURRENT_REACTOR.get().expect("Called outside an runtime context")
    //means not reactor not yet setup. aka spawn not called yet
}

//ofcourse the reactor should expose functions that allow futures to register interest in some I/O

impl Reactor {
    pub fn register(&self, source: &mut impl Source, interest: Interest, id: usize, waker: &Waker) {
        //add to registry
        self.registry.register(source, mio::Token(id), interest);
        //now register waker
        self.wakers.lock().unwrap().insert(id, waker.clone());
    }

    //our runtime does not support work-stealing as of not, which means futures won't move from one executor to another -> the sleeping thread which is supposed to be woken up for our future will never change -> waker will remain valid through out
    //however in a runtime that does support work stealing, in worst case, a future might change the hosting executor on between every poll.
    //once executor A (running on thread A) polls a future and recieved notReady, it might put it back in its tasks. Another executor B (running on thread B) might steal this task from A. once B polls it and is has no futures ready to poll,
    //and is therefore parked, but the waker passed to the future while polling will remain same, even when it has to be updated, since now another thread needs to be woke up for this future.
    //therefore let us expose a function that allows updating the waker

    //the poll function of a future is responsible for making sure interest is registered on the latest waker, which can be achieved by
    //always calling update_waker before ending a poll (returning NotReady)
    pub fn update_waker(&self, id: usize, waker: &Waker) {
        //automatically updates
        self.wakers.lock().unwrap().insert(id, waker.clone());
    }
    pub fn next_id(&self) -> usize {
        //give out next id, by incrementing. atomically safe.
        self.next_id.fetch_add(1, Relaxed)
    }

    //naturally, allow to unregister interest on a source if its associated future has already returned ready
    pub fn unregister(&self, source: &mut impl Source, id: usize) {
        //if we unregistered with mio we wont recieve any associated events, so why even keep the waker in memory ?
        self.wakers.lock().unwrap().remove(&id);
        //now actually unregister with mio
        self.registry.deregister(source);

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
