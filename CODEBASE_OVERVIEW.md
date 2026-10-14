# YARR Codebase Overview

This document follows the running example from startup through an HTTP response,
then describes the supporting modules and exploratory source files. It is an
overview, not a replacement for the inline comments: use those comments to
study the implementation details.

For YARR's purpose and limitations, see the [main README](./README.md).

## 1. Start at the executable

Cargo builds `src/main.rs` as the program entry point. Its `main` function
creates an executor with `runtime::init()` and passes `async_main()` to
`block_on`.

`async_main` is an ordinary Rust async function. It awaits two HTTP requests in
sequence and prints each response. The Rust compiler transforms this function
into a future state machine; YARR drives that future using Rust's standard
`Future` interface.

## 2. Set up the runtime and executors

The runtime module starts the reactor and returns an `Executor` for the current
thread. The `block_on` call runs the top-level `async_main` future on it.

The runtime also exposes `join_all` for multiple top-level futures. It runs the
first future on the current executor and creates a separate operating-system
thread and executor for each remaining future, then joins those threads. The
example helper `join_tester` in `src/main.rs` illustrates this API, but the
normal `main` path does not call it. This is multiple executors, not a shared
task pool: each executor drives its own top-level future. There is no work
stealing or migration of futures between executors.

The executor implementation keeps its working state in thread-local storage.
When `block_on` receives a future, it pins it in a `Box`, polls it once, and
then stores it as a task if it is still pending. Tasks are represented as
`Pin<Box<dyn Future<Output = ()>>>`; the executor loop removes ready task IDs
from a queue, polls each future using a `Context`, and then either:

- stores it again if it returned `Poll::Pending`, or
- considers it complete if it returned `Ready`.

When there is no ready task but unfinished work remains, the executor parks
its thread. A waker can enqueue a task ID and unpark that thread. When no tasks
remain, `block_on` returns.

Before polling, `block_on` pins the top-level future in a `Box`. The task map
keeps that pinned allocation, and the executor polls through its pinned
reference each time the task is resumed. This allows futures that rely on
remaining at a stable memory address after polling begins.

### Why top-level futures must return `()`

The executor's task storage has one fixed, type-erased task type:
`Pin<Box<dyn Future<Output = ()>>>`. Consequently, `block_on` and `join_all`
require their top-level futures to have `Output = ()`; the runtime does not
collect or return a future's result.

This design is tied to the executor state being held in a `thread_local!`
static (`CURRENT_EXEC`). It is the executor's shared state and task registry
that live in static storage, not the `Executor` handle itself. Rust does not
allow generic statics, so this registry cannot vary its stored future type by
output type. Choosing unit output gives the static task registry a single
uniform trait object type. Runtimes that need to return task results generally
expose a result-bearing handle, such as a join handle, rather than making the
static executor storage generic over each output type.

Type erasure at the runtime's task-storage boundary is not unique to YARR:
Tokio also erases concrete future types internally. Its public task API can
still accept different future and output types, with results retrieved through
a [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html).

## 3. Poll the HTTP future

`Http::get` in `src/http.rs` creates an `HttpGetFuture`. On its first poll, the
future opens a TCP connection to `127.0.0.1:8080`, makes the stream
non-blocking, sends a basic HTTP GET request, and registers the stream for
readable events with the reactor. This future implements Rust's
`std::future::Future` directly.

It then attempts to read the response:

- If bytes are available, it appends them to its buffer and keeps reading.
- If the socket would block, it updates the reactor's waker for this operation
  and returns `Poll::Pending`.
- When the peer closes the connection, it converts the buffered bytes to a
  string, unregisters the stream, and returns `Poll::Ready`.

The request is intentionally minimal. It is only a demonstration of polling
around non-blocking I/O, not a general HTTP client.

## 4. Wake the waiting executor

The reactor in `src/runtime/reactor.rs` runs a `mio` poll loop on its own
thread. The HTTP future registers I/O interest with a unique token and a clone
of the current task's standard-library `Waker`, obtained through its `Context`.

When `mio` reports an event for that token, the reactor looks up the associated
waker and wakes it. YARR implements Rust's `Wake` trait for its `CustomWaker`;
that implementation puts the task ID into the executor's ready queue and
unparks its thread. The executor loop then polls the task again, allowing the
HTTP future to continue reading.

YARR uses Rust's own `Future`, `Poll`, `Context`, `Waker`, and `Wake` APIs. Its
`CustomWaker` is the runtime-specific scheduling behavior plugged into the
standard waker system, not a separate future or waker abstraction.

The executor and reactor have separate roles: executors run future code, while
the reactor waits for I/O readiness and signals which executor should resume.

## 5. How the modules fit together

- `src/runtime.rs` exposes the executor, waker, reactor accessor, and runtime
  initialization function.
- `src/runtime/executor.rs` owns the per-thread task loop, pinned task storage,
  ready queue, and `Wake` implementation.
- `src/runtime/reactor.rs` owns the `mio` event loop and maps I/O tokens to
  standard-library wakers.
- `src/http.rs` implements a standard-library future around a non-blocking
  socket.
- `src/main.rs` demonstrates Rust async functions and includes a helper for
  running multiple unit-output futures across executors.

## 6. Other source files

`src/future.rs`, `src/main3.rs`, `src/main5.rs`, `src/main_corofied.rs`, and
`src/main5_corofied.rs` are additional exploration snapshots of earlier
examples and coroutine-style state machines. The active runtime modules do not
use the old custom future trait in `src/future.rs`. Cargo does not select these
files as the executable entry point; the walkthrough above describes
`src/main.rs` and the runtime modules it uses. Some snapshots use earlier APIs
and may not build as standalone binaries.

## 7. Important constraints

The example assumes an HTTP server is already listening at
`127.0.0.1:8080`. It is educational code, not production infrastructure. The
runtime uses Rust's standard future and waker APIs and pins tasks before polling,
but it does not implement work stealing; error handling and lifecycle
management are intentionally minimal. Top-level futures driven by its executor
must return `()`. The [main README](./README.md) has the broader cautions and
further reading links.
