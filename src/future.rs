use std::pin::Pin;

use crate::runtime::Waker;


pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}