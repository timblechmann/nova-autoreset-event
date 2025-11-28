use std::io;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;

use libc::{c_void, pipe, read, write};

/// An autoreset event.
///
/// See the [module-level documentation](..) for more information.
#[derive(Debug)]
pub struct AutoResetEvent {
    fds: [OwnedFd; 2],
}

impl AutoResetEvent {
    /// Creates a new autoreset event.
    pub fn new() -> io::Result<Self> {
        let mut fds_raw = [0; 2];
        let res = unsafe { pipe(fds_raw.as_mut_ptr()) };

        if res == -1 {
            Err(io::Error::last_os_error())
        } else {
            let fds = unsafe {
                [
                    OwnedFd::from_raw_fd(fds_raw[0]),
                    OwnedFd::from_raw_fd(fds_raw[1]),
                ]
            };
            Ok(Self { fds })
        }
    }

    /// Waits for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return immediately and
    /// reset the event to the unsignalled state. Otherwise, it will block until another thread
    /// signals the event.
    pub fn wait(&self) {
        let mut buf = [0u8; 1];
        let res = unsafe { read(self.fds[0].as_raw_fd(), buf.as_mut_ptr() as *mut c_void, 1) };

        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("read failed with error {}", err);
        }
    }

    /// Tries to wait for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return `true` immediately
    /// and reset the event to the unsignalled state. Otherwise, it will return `false` immediately.
    pub fn try_wait(&self) -> bool {
        self.try_wait_for(Duration::from_millis(0))
    }

    /// Tries to wait for the event to be signalled for a specified duration.
    ///
    /// If the event is already in the signalled state, this function will return `true` immediately
    /// and reset the event to the unsignalled state. If the event is signalled within the timeout,
    /// it will return `true`. Otherwise, it will return `false`.
    pub fn try_wait_for(&self, timeout: Duration) -> bool {
        let mut pollfd = libc::pollfd {
            fd: self.fds[0].as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let millis = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
        let ret = unsafe { libc::poll(&mut pollfd, 1, millis) };

        if ret == -1 {
            let err = io::Error::last_os_error();
            panic!("poll failed with error {}", err);
        }

        if ret > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            // Read the value to reset the event
            let mut buf = [0u8; 1];
            let res = unsafe { read(self.fds[0].as_raw_fd(), buf.as_mut_ptr() as *mut c_void, 1) };
            if res == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    return false;
                }
                panic!("read failed with error {}", err);
            }
            true
        } else {
            false
        }
    }

    /// Signals the event.
    ///
    /// If there is a thread waiting on the event, it will be woken up and the event will be reset
    /// to the unsignalled state. If there are no threads waiting, the event will remain in the
    /// signalled state until a thread waits on it.
    pub fn signal(&self) {
        let buf = [0u8; 1];
        let res = unsafe { write(self.fds[1].as_raw_fd(), buf.as_ptr() as *const c_void, 1) };

        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("write failed with error {}", err);
        }
    }
}

impl AsRawFd for AutoResetEvent {
    fn as_raw_fd(&self) -> RawFd {
        self.fds[0].as_raw_fd()
    }
}

impl AsFd for AutoResetEvent {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fds[0].as_fd()
    }
}


// It is safe to send an autoreset event to another thread. The underlying file descriptors are
// kernel objects that can be used from any thread.
unsafe impl Send for AutoResetEvent {}

// It is safe to share an autoreset event between threads. The underlying file descriptors are
// kernel objects that are thread-safe.
unsafe impl Sync for AutoResetEvent {}
