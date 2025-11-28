#![cfg(windows)]

use std::io;
use std::os::windows::io::{
    AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle, RawHandle,
};
use std::ptr;
use std::time::Duration;

use winapi::shared::minwindef::{FALSE, TRUE};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::synchapi::{CreateEventW, SetEvent, WaitForSingleObject};
use winapi::um::winbase::WAIT_OBJECT_0;
use winapi::um::winnt::HANDLE;

/// An autoreset event.
///
/// See the [module-level documentation](..) for more information.
#[derive(Debug)]
pub struct AutoResetEvent {
    handle: OwnedHandle,
}

impl AutoResetEvent {
    /// Creates a new autoreset event.
    pub fn new() -> io::Result<Self> {
        let handle = unsafe { CreateEventW(ptr::null_mut(), FALSE, FALSE, ptr::null()) };

        if handle == ptr::null_mut() || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self {
                handle: unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) },
            })
        }
    }

    /// Waits for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return immediately and
    /// reset the event to the unsignalled state. Otherwise, it will block until another thread
    /// signals the event.
    pub fn wait(&self) {
        let res = unsafe { WaitForSingleObject(self.handle.as_raw_handle() as HANDLE, u32::MAX) };

        if res != WAIT_OBJECT_0 {
            // This should not happen
            let err = unsafe { GetLastError() };
            panic!("WaitForSingleObject failed with error {}", err);
        }
    }

    /// Tries to wait for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return `true` immediately
    /// and reset the event to the unsignalled state. Otherwise, it will return `false` immediately.
    pub fn try_wait(&self) -> bool {
        let res = unsafe { WaitForSingleObject(self.handle.as_raw_handle() as HANDLE, 0) };

        if res == WAIT_OBJECT_0 {
            true
        } else if res == WAIT_TIMEOUT {
            false
        } else {
            // This should not happen
            let err = unsafe { GetLastError() };
            panic!("WaitForSingleObject failed with error {}", err);
        }
    }

    /// Tries to wait for the event to be signalled for a specified duration.
    ///
    /// If the event is already in the signalled state, this function will return `true` immediately
    /// and reset the event to the unsignalled state. If the event is signalled within the timeout,
    /// it will return `true`. Otherwise, it will return `false`.
    pub fn try_wait_for(&self, timeout: Duration) -> bool {
        let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
        let res = unsafe { WaitForSingleObject(self.handle.as_raw_handle() as HANDLE, millis) };

        if res == WAIT_OBJECT_0 {
            true
        } else if res == WAIT_TIMEOUT {
            false
        } else {
            // This should not happen
            let err = unsafe { GetLastError() };
            panic!("WaitForSingleObject failed with error {}", err);
        }
    }

    /// Signals the event.
    ///
    /// If there is a thread waiting on the event, it will be woken up and the event will be reset
    /// to the unsignalled state. If there are no threads waiting, the event will remain in the
    /// signalled state until a thread waits on it.
    pub fn signal(&self) {
        let res = unsafe { SetEvent(self.handle.as_raw_handle() as HANDLE) };

        if res != TRUE {
            // This should not happen
            let err = unsafe { GetLastError() };
            panic!("SetEvent failed with error {}", err);
        }
    }
}

impl AsRawHandle for AutoResetEvent {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.as_raw_handle()
    }
}

impl AsHandle for AutoResetEvent {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.handle.as_handle()
    }
}

// It is safe to send an autoreset event to another thread. The underlying handle is a kernel
// object that can be used from any thread.
unsafe impl Send for AutoResetEvent {}

// It is safe to share an autoreset event between threads. The underlying handle is a kernel
// object that is thread-safe.
unsafe impl Sync for AutoResetEvent {}
