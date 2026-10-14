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

- Rust's standard `std::future::Future` trait and task-waking APIs.
- An executor that polls ready futures and parks its thread while tasks wait.
- A `mio`-based reactor that registers I/O interest and wakes the associated
  executor when events arrive.
- A multiple-executor helper that can drive separate top-level futures on
  separate executor threads.

This is not a work-stealing runtime. Futures are not moved between executors,
and the design does not implement task migration or the broader coordination
needed for a production runtime. The executor stores pinned futures and polls
them through Rust's standard `Pin`, `Context`, `Poll`, and `Waker` APIs. Its
custom `Wake` implementation connects Rust's waker mechanism to this runtime's
ready queue and parked threads.

## Run the example

The example issues HTTP requests to `127.0.0.1:8080`. Start a compatible HTTP
server on that address before running it:

```sh
cargo run
```

The executable makes two HTTP requests sequentially and prints their responses.
Without a server listening locally on port `8080`, the example will not work as
intended. The source also contains a `join_all` helper for running multiple
unit-output futures on separate executor threads; see the
[codebase overview](./CODEBASE_OVERVIEW.md).

## Project layout

- `src/runtime/executor.rs` — task polling, ready queue, and thread waker.
- `src/runtime/reactor.rs` — I/O event loop and waker registration.
- `src/http.rs` — a minimal non-blocking HTTP GET future.
- `src/main.rs` — async entry point and a multi-executor example helper.

For a high-level walkthrough of how these pieces work together, see the
[codebase overview](./CODEBASE_OVERVIEW.md).

## Further reading

- [*Asynchronous Programming in Rust* — Carl Fredrik Samson](https://github.com/PacktPublishing/Asynchronous-Programming-in-Rust)
- [The Rust Book: Asynchronous Programming](https://doc.rust-lang.org/book/ch17-00-async-await.html)
- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html) — the standard-library future trait for comparison
- [`mio` documentation](https://docs.rs/mio/0.8/mio/) — the I/O event library used by this project
