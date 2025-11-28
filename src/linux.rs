// The initial value of the eventfd
const EFD_INITIAL_VALUE: u32 = 0;

use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::time::Duration;

/// An autoreset event.
///
/// See the [module-level documentation](..) for more information.
#[derive(Debug)]
pub struct AutoResetEvent {
    fd: OwnedFd,
}

impl AutoResetEvent {
    /// Creates a new autoreset event.
    pub fn new() -> std::io::Result<Self> {
        let fd = unsafe { libc::eventfd(EFD_INITIAL_VALUE, libc::EFD_CLOEXEC) };

        if fd == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self {
                fd: unsafe { OwnedFd::from_raw_fd(fd) },
            })
        }
    }

    /// Waits for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return immediately and
    /// reset the event to the unsignalled state. Otherwise, it will block until another thread
    /// signals the event.
    pub fn wait(&self) {
        let mut value: u64 = 0;
        let ret = unsafe {
            libc::read(
                self.fd.as_raw_fd(),
                &mut value as *mut _ as *mut libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };

        if ret == -1 {
            // This should not happen
            let err = std::io::Error::last_os_error();
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
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let millis = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
        let ret = unsafe { libc::poll(&mut pollfd, 1, millis) };

        if ret == -1 {
            let err = std::io::Error::last_os_error();
            panic!("poll failed with error {}", err);
        }

        if ret > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            // Read the value to reset the event
            let mut value: u64 = 0;
            let ret = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    &mut value as *mut _ as *mut libc::c_void,
                    std::mem::size_of::<u64>(),
                )
            };
            if ret == -1 {
                // This might happen if another thread stole the signal between poll and read,
                // but for an autoreset event, that's expected behavior in a race.
                // However, if we are the only one waiting (or if we want to report success),
                // we should consider what to return.
                // If read fails with EAGAIN/EWOULDBLOCK, it means it wasn't ready.
                // But poll said it was.
                // For now, let's assume if poll returns > 0, we should be able to read.
                // But to be safe against spurious wakeups or race conditions:
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
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
        let value: u64 = 1;
        let ret = unsafe {
            libc::write(
                self.fd.as_raw_fd(),
                &value as *const _ as *const libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };

        if ret == -1 {
            // This should not happen
            let err = std::io::Error::last_os_error();
            panic!("write failed with error {}", err);
        }
    }
}

impl AsRawFd for AutoResetEvent {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsFd for AutoResetEvent {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}


// It is safe to send an autoreset event to another thread. The underlying file descriptor is a
// kernel object that can be used from any thread.
unsafe impl Send for AutoResetEvent {}

// It is safe to share an autoreset event between threads. The underlying file descriptor is a
// kernel object that is thread-safe.
unsafe impl Sync for AutoResetEvent {}
