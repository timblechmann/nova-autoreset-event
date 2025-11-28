#![cfg_attr(docsrs, feature(doc_cfg))]

//! An autoreset event primitive.
//!
//! An autoreset event is a synchronization primitive that can be used to signal between threads.
//! When a thread waits on an unsignalled event, it
//! will block until another thread signals the event. When an event is signalled, it will wake up
//! exactly one waiting thread and then automatically reset to the unsignalled state. If an event
//! is signalled and no threads are waiting, it will remain in the signalled state until a thread
//! waits on it.
//!
//! This crate provides a cross-platform implementation of an autoreset event. It is implemented
//! using Win32 `CreateEvent` on Windows, `eventfd` on Linux, `kqueue` on macOS/BSD, and a pipe-based
//! fallback on other platforms. The `eventfd`, `kqueue` and `pipe` implementations implement `AsFd` and `AsRawFd`,
//! while the Win32 implementation implements `AsHandle` and `AsRawHandle`.

// Set on linux/android
#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))
))]
mod linux;
#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))
))]
pub use linux::AutoResetEvent;

// Set on macos/ios/freebsd/netbsd/openbsd/dragonfly
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod macos;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub use macos::AutoResetEvent;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::AutoResetEvent;

#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_os = "linux",
        target_os = "android"
    ))
))]
mod pipe;
#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_os = "linux",
        target_os = "android"
    ))
))]
pub use pipe::AutoResetEvent;
