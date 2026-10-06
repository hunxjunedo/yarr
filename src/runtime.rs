use std::sync::OnceLock;
use crate::future::{Future, PollState::{Ready, NotReady}};
use mio::{Events, Poll, Registry};
mod reactor;
mod executor;

static REGISTRY : OnceLock<Registry> = OnceLock::new();
pub fn registry() -> &'static Registry {
    //will through error if registry uninitialized
    REGISTRY.get().expect("Called outside a runtime context")
}

pub struct Runtime {
    poll: Poll
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().expect("poll not supported on this OS");
        let reg = poll.registry().try_clone().unwrap();
        REGISTRY.set(reg).unwrap(); 
        Self { poll }
    }

    pub fn block_on(&mut self, fut: impl Future<Output = String>) {
        //before we register interest and sleep, we need to find the socket
        //poll the future we recieved. If its ready immideatily, return immideatily.
        let mut future = fut;
        loop{match future.poll() {
            //we still keep looping but a little better. If we get a noready, we block! 
            NotReady => {
                println!("do other work");
                //this task is busy. block here.
                let mut events = Events::with_capacity(100);
                self.poll.poll(&mut events, None).unwrap();
                //this flow continue each time this unblocks; aka each time some event occurs on subscribees
            },
            Ready(v) => break
        }}
    }
}


