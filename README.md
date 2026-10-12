# YARR

**YARR** means **Yet Another Rust Runtime**. It is a small, heavily commented
toy runtime for learning how asynchronous runtimes work.

The project is intended to be read alongside *[Asynchronous Programming in
Rust](https://github.com/PacktPublishing/Asynchronous-Programming-in-Rust)* by
Carl Fredrik Samson. The book provides the concepts and background; YARR is a
place to explore a simple implementation.

> [!WARNING]
> YARR is educational only and is **not suitable for production**. It is
> incomplete and deliberately omits important runtime features and safeguards.
> Do not rely on it for robust networking, cancellation, error handling, or
> general-purpose asynchronous execution.

## What it demonstrates

- A custom `Future` trait and `PollState` type, rather than
  `std::future::Future`.
- An executor that polls ready futures and parks its thread while tasks wait.
- A `mio`-based reactor that registers I/O interest and wakes the associated
  executor when events arrive.
- A multiple-executor architecture: the example drives separate top-level
  futures on separate executor threads.

This is not a work-stealing runtime. Futures are not moved between executors,
and the design does not implement task migration or the broader coordination
needed for a production runtime. It also has no pinning yet; its custom future
interface uses `&mut self` directly.

## Run the example

The example issues HTTP requests to `127.0.0.1:8080`. Start a compatible HTTP
server on that address before running it:

```sh
cargo run
```

The executable launches multiple requests and prints the last line of each
response. Without a server listening locally on port `8080`, the example will
not work as intended.

## Project layout

- `src/future.rs` — custom future and polling types.
- `src/runtime/executor.rs` — task polling, ready queue, and thread waker.
- `src/runtime/reactor.rs` — I/O event loop and waker registration.
- `src/http.rs` — a minimal non-blocking HTTP GET future.
- `src/main.rs` — hand-written state-machine future and multi-executor demo.

For a high-level walkthrough of how these pieces work together, see the
[codebase overview](./CODEBASE_OVERVIEW.md).

## Further reading

- [*Asynchronous Programming in Rust* — Carl Fredrik Samson](https://github.com/PacktPublishing/Asynchronous-Programming-in-Rust)
- [The Rust Book: Asynchronous Programming](https://doc.rust-lang.org/book/ch17-00-async-await.html)
- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html) — the standard-library future trait for comparison
- [`mio` documentation](https://docs.rs/mio/0.8/mio/) — the I/O event library used by this project
