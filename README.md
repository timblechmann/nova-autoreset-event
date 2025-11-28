# `nova-autoreset-event`

[![CI](https://github.com/timblechmann/nova-autoreset-event/workflows/CI/badge.svg)](https://github.com/timblechmann/nova-autoreset-event/actions)
[![Crates.io](https://img.shields.io/crates/v/nova-autoreset-event.svg)](https://crates.io/crates/nova-autoreset-event)
[![Documentation](https://docs.rs/nova-autoreset-event/badge.svg)](https://docs.rs/nova-autoreset-event)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A cross-platform autoreset event primitive. An autoreset event is a synchronization primitive that
can be used to signal between threads.
When a thread waits on an unsignalled event, it will block until another thread signals the event.
When an event is signalled, it will wake up exactly one waiting thread and then automatically reset
to the unsignalled state. If an event is signalled and no threads are waiting, it will remain in
the signalled state until a thread waits on it.

This crate provides a cross-platform implementation of an autoreset event. It is implemented
using Win32 `CreateEvent` on Windows, `eventfd` on Linux, `kqueue` on macOS/BSD, and a pipe-based
fallback on other platforms. The `eventfd`, `kqueue` and `pipe` implementations implement `AsFd` and `AsRawFd`,
while the Win32 implementation implements `AsHandle` and `AsRawHandle`.

## Usage

```rust
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use nova_autoreset_event::AutoResetEvent;

fn main() {
    let event = Arc::new(AutoResetEvent::new().unwrap());

    let thread = {
        let event = event.clone();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            event.signal();
        })
    };

    event.wait();

    thread.join().unwrap();
}
```

## Tokio integration

On Unix, the `eventfd`, `kqueue`, and `pipe` implementations expose the underlying file descriptor
via the `AsRawFd` trait, allowing integration with Tokio's async I/O using `AsyncFd`.

Example:

```rust
use std::os::unix::io::AsRawFd;
use tokio::io::unix::AsyncFd;

let event = nova_autoreset_event::AutoResetEvent::new().unwrap();
let async_fd = AsyncFd::new(event.as_raw_fd()).unwrap();

// Wait asynchronously for the event to be signalled
let mut guard = async_fd.readable().await.unwrap();
guard.clear_ready();

// Use wait() to properly consume the signal
// wait() will not block because we know the event is signaled
event.wait();
```

Note: On Windows, the Win32 `HANDLE` cannot be used with Tokio's async I/O, so you should use blocking `wait()` or spawn a blocking task.
