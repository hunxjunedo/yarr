use std::{fmt::Write, pin::Pin};
use crate::{
    future::{Future, PollState},
    runtime::Waker,
};

mod future;
mod http;
mod runtime;

fn main() {
    let mut runtime = runtime::init();
    runtime.block_on(async_main());
}



fn async_main() -> impl Future<Output = String> {
    Coroutine0::new()
}

enum State0 {
    Start,
    Wait1(Pin<Box<dyn Future<Output = String>>>),
    Wait2(Pin<Box<dyn Future<Output = String>>>),
    Resolved,
}

struct Coroutine0 {
    state: State0,
    state_data: State0Data
}
#[derive(Default)]
struct State0Data {
    buffer: Option<String>,
    writer: Option<*mut String>
}

impl Coroutine0 {
    fn new() -> Self {
        Self {
            state: State0::Start,
            state_data: State0Data::default()
        }
    }
}

impl Future for Coroutine0 {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, waker: &Waker) -> PollState<Self::Output> {
        loop {
            match self.state {
                State0::Start => {
                    // ---- Code you actually wrote ----
                    self.state_data.buffer = Some(String::from("\nBUFFER:\n----\n"));
                    self.state_data.writer = Some(self.state_data.buffer.as_mut().unwrap());
                    println!("Program starting");

                    
                    let fut1 = Box::pin(http::Http::get("/600/HelloAsyncAwait".to_string()));
                    self.state = State0::Wait1(fut1);
                   
                }

                State0::Wait1(ref mut f1) => {
                    match f1.as_mut().poll(waker) {
                        PollState::Ready(txt) => {
                            //get the writer from the stack, lets see what happens if future is moved in between polls.
                            println!("{:?}", unsafe{& *self.state_data.writer.clone().unwrap()});
                            let writer = unsafe {&mut *self.state_data.writer.take().unwrap()};
                            writeln!(writer, "{txt}").unwrap();

                            
                            let fut2 =
                                Box::pin(http::Http::get("/400/HelloAsyncAwait".to_string()));
                            self.state = State0::Wait2(fut2);
                            //move the writer back to the state once you are done
                            self.state_data.writer = Some(writer);
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Wait2(ref mut f2) => {
                    match f2.as_mut().poll(waker) {
                        PollState::Ready(txt) => {
                            //retrieve data from the state
                            let writer = unsafe {&mut *self.state_data.writer.take().unwrap()};
                            writeln!(writer, "{txt}").unwrap();
                            let buffer = self.state_data.buffer.take().unwrap();
                            println!("{}", buffer);

                            // ---------------------------------
                            self.state = State0::Resolved;
                            //all data already freed when we called take()
                            //now uncomment the following line
                            // println!("{:?}", self.state_data.buffer.as_ref());
                            //the above line should print None, which indicates mem freed.
                           //what about the pointer ? lets remove it also (though not that important)
                           let _ = self.state_data.writer.take();
                           //println!("{:?}", self.state_data.writer);
                           //above line should print None
                           

                            break PollState::Ready(String::new());
                            
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Resolved => panic!("Polled a resolved future"),
            }
        }
    }
}
