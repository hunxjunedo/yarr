use mio::{Interest};
use crate::{runtime::{reactor}};
use std::{future::Future, task::{Context, Poll}};
use std::{io::{ErrorKind, Read, Write}, pin::Pin};

fn get_req(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\n\
             Host: localhost\r\n\
             Connection: close\r\n\
             \r\n"
    )
}

pub struct Http;

impl Http {
    pub fn get(path: String) -> impl Future<Output = String> {
        HttpGetFuture::new(&path)
    }
}

struct HttpGetFuture {
    stream: Option<mio::net::TcpStream>,
    buffer: Vec<u8>,
    path: String,
    id: usize
}

impl HttpGetFuture {
    fn new(path: &str) -> Self {
        Self {
            stream: None,
            buffer: vec![],
            path: path.to_string(),
            id: reactor().next_id()
        }
    }

    fn write_request(&mut self) {
        let stream = std::net::TcpStream::connect("127.0.0.1:8080").unwrap();
        stream.set_nonblocking(true).unwrap();
        let mut stream = mio::net::TcpStream::from_std(stream);
        stream.write_all(get_req(&self.path).as_bytes()).unwrap();
        self.stream = Some(stream);
    }
}

impl Future for HttpGetFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        println!("POLL!!!");
        if self.stream.is_none() {
            println!("FIRST POLL - START OPERATION");
            let id = self.id;
            self.as_mut().write_request();
            let stream = self.stream.as_mut().unwrap();
            //updated method of registering interest
            reactor().register(stream, Interest::READABLE, id, cx);
            // return Poll::Pending;
            // principle of polling: do not yield until you absolutely have to wait on something. You dont necessirily have to wait on stream, UNLESS it says would block.
        }

        let mut buff = vec![0u8; 4096];
        loop {
            match self.stream.as_mut().unwrap().read(&mut buff) {
                Ok(0) => {
                    let id = self.id;
                    let s = String::from_utf8_lossy(&self.buffer).into_owned();
                    //we are returning here too, we are no longer interested in this stream, so un_register
                    reactor().unregister(self.stream.as_mut().unwrap(), id);
                    break Poll::Ready(s);
                }
                Ok(n) => {
                    self.buffer.extend(&buff[0..n]);
                    continue;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    //we are about to return, so lets make sure latest waker is associated, check the comments on top of function implementation for more details
                    reactor().update_waker(self.id, cx);
                    break Poll::Pending;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    continue;
                }
                Err(e) => panic!("{e:?}"),
            }
        }
    }
}
