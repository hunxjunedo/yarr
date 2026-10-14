# YARR Codebase Overview

This document follows the running example from startup through an HTTP response,
then describes the supporting modules and exploratory source files. It is an
overview, not a replacement for the inline comments: use those comments to
study the implementation details.

For YARR's purpose and limitations, see the [main README](./README.md).

## 1. Start at the executable

Cargo builds `src/main.rs` as the program entry point. Its `main` function
creates twelve `request` futures and passes them to `join_all`.

Each `request` is a manually written state machine. `Coroutine0` begins in
`Start`, constructs the request path, and creates an HTTP future. In `Wait1`, it
polls that future. If the HTTP operation is still waiting, the coroutine also
waits; when the HTTP future completes, it prints the final response line and
marks itself `Resolved`.

This state machine is the educational stand-in for what a compiler-generated
async state machine might do. It is written out explicitly so the transitions
between suspension and resumption are visible.

## 2. Set up the runtime and executors

`join_all` calls `runtime::init()`. The runtime module starts the reactor and
returns an `Executor` for the current thread. That executor handles the first
request. Each remaining request gets a separate operating-system thread with
its own executor, and `join_all` joins those threads before returning.

This is multiple executors, not a shared task pool: each executor drives its
own top-level future. There is no work stealing or migration of futures between
executors.

The executor implementation keeps its working state in thread-local storage.
When `block_on` receives a future, it registers it as a task and marks it ready
for its first poll. The executor loop removes ready task IDs from a queue,
polls the corresponding future, and then either:

- stores it again if it returned `NotReady`, or
- considers it complete if it returned `Ready`.

When there is no ready task but unfinished work remains, the executor parks
its thread. A waker can enqueue a task ID and unpark that thread. When no tasks
remain, `block_on` returns.

Before polling, `block_on` pins the top-level future in a `Box`. The task map
keeps that pinned allocation, and the executor polls through its pinned
reference each time the task is resumed. This lets the custom future interface
use `Pin<&mut Self>` and supports futures that rely on remaining at a stable
memory address after polling begins.

## 3. Poll the HTTP future

`Http::get` in `src/http.rs` creates an `HttpGetFuture`. On its first poll, the
future opens a TCP connection to `127.0.0.1:8080`, makes the stream
non-blocking, sends a basic HTTP GET request, and registers the stream for
readable events with the reactor.

It then attempts to read the response:

- If bytes are available, it appends them to its buffer and keeps reading.
- If the socket would block, it updates the reactor's waker for this operation
  and returns `NotReady`.
- When the peer closes the connection, it converts the buffered bytes to a
  string, unregisters the stream, and returns `Ready`.

The request is intentionally minimal. It is only a demonstration of polling
around non-blocking I/O, not a general HTTP client.

## 4. Wake the waiting executor

The reactor in `src/runtime/reactor.rs` runs a `mio` poll loop on its own
thread. The HTTP future registers I/O interest with a unique token and a clone
of the current executor's waker.

When `mio` reports an event for that token, the reactor looks up the associated
waker and calls it. The waker puts the task ID into that executor's ready queue
and unparks its thread. The executor loop then polls the task again, allowing
the HTTP future to continue reading.

The executor and reactor have separate roles: executors run future code, while
the reactor waits for I/O readiness and signals which executor should resume.

## 5. How the modules fit together

- `src/future.rs` defines YARR's own `Future` trait and its `Ready` /
  `NotReady` polling result. Its `poll` method takes `Pin<&mut Self>`, making
  pinning part of the custom future interface. It is separate from the standard
  library's future trait.
- `src/runtime.rs` exposes the executor, waker, reactor accessor, and runtime
  initialization function.
- `src/runtime/executor.rs` owns the per-thread task loop, ready queue, and
  waker behavior.
- `src/runtime/reactor.rs` owns the `mio` event loop and maps I/O tokens to
  wakers.
- `src/http.rs` connects the custom future interface to a non-blocking socket.
- `src/main.rs` demonstrates a hand-written async-style state machine and
  multiple executors.

## 6. Other source files

`src/main3.rs`, `src/main5.rs`, `src/main_corofied.rs`, and
`src/main5_corofied.rs` are additional exploration snapshots of earlier
examples and coroutine-style state machines. Cargo does not select these files
as the executable entry point; the walkthrough above describes `src/main.rs`
and the runtime modules it uses. Some snapshots use earlier APIs and may not
build as standalone binaries.

## 7. Important constraints

The example assumes an HTTP server is already listening at
`127.0.0.1:8080`. It is educational code, not production infrastructure. In
particular, although futures are pinned before polling, the runtime does not
implement work stealing, and its error handling and lifecycle management are
intentionally minimal. The [main README](./README.md) has the broader cautions
and further reading links.
