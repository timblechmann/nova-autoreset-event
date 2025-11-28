use std::io;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::time::Duration;

use libc::{EV_ADD, EV_CLEAR, EV_DELETE, EVFILT_USER, c_void, kevent, kqueue, pipe, write};

#[macro_export]
macro_rules! EV_SET {
    ($ev:expr, $ident:expr, $filter:expr, $flags:expr, $fflags:expr, $data:expr, $udata:expr) => {
        $ev.ident = $ident as libc::uintptr_t;
        $ev.filter = $filter as libc::c_short;
        $ev.flags = $flags as libc::c_ushort;
        $ev.fflags = $fflags as libc::c_uint;
        $ev.data = $data as libc::intptr_t;
        $ev.udata = $udata as *mut libc::c_void;
    };
}


/// An autoreset event.
///
/// See the [module-level documentation](..) for more information.
#[derive(Debug)]
pub struct AutoResetEvent {
    kq: OwnedFd,
    ident: usize,
    fds: [OwnedFd; 2],
}

impl AutoResetEvent {
    /// Creates a new autoreset event.
    pub fn new() -> io::Result<Self> {
        let kq_raw = unsafe { kqueue() };
        if kq_raw == -1 {
            return Err(io::Error::last_os_error());
        }
        let kq = unsafe { OwnedFd::from_raw_fd(kq_raw) };

        let mut fds_raw = [0; 2];
        if unsafe { pipe(fds_raw.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
            // kq is dropped here, closing the fd
        }
        let fds = unsafe {
            [
                OwnedFd::from_raw_fd(fds_raw[0]),
                OwnedFd::from_raw_fd(fds_raw[1]),
            ]
        };

        let event = Self { kq, ident: 1, fds };

        // Add a new user event to the kqueue.
        let mut ke: libc::kevent = unsafe { std::mem::zeroed() };
        EV_SET!(
            &mut ke,
            event.ident,
            EVFILT_USER,
            EV_ADD | EV_CLEAR,
            0,
            0,
            ptr::null_mut()
        );

        let res = unsafe {
            kevent(
                event.kq.as_raw_fd(),
                &ke,
                1,
                ptr::null_mut(),
                0,
                ptr::null(),
            )
        };
        if res == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(event)
    }

    /// Waits for the event to be signalled.
    ///
    /// If the event is already in the signalled state, this function will return immediately and
    /// reset the event to the unsignalled state. Otherwise, it will block until another thread
    /// signals the event.
    pub fn wait(&self) {
        let mut ke: libc::kevent = unsafe { std::mem::zeroed() };
        let res = unsafe { kevent(self.kq.as_raw_fd(), ptr::null(), 0, &mut ke, 1, ptr::null()) };

        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("kevent failed with error {}", err);
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
        let mut ke: libc::kevent = unsafe { std::mem::zeroed() };
        let ts = libc::timespec {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_nsec: timeout.subsec_nanos() as libc::c_long,
        };
        let res = unsafe { kevent(self.kq.as_raw_fd(), ptr::null(), 0, &mut ke, 1, &ts) };

        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("kevent failed with error {}", err);
        }

        res > 0
    }

    /// Signals the event.
    ///
    /// If there is a thread waiting on the event, it will be woken up and the event will be reset
    /// to the unsignalled state. If there are no threads waiting, the event will remain in the
    /// signalled state until a thread waits on it.
    pub fn signal(&self) {
        let mut ke: libc::kevent = unsafe { std::mem::zeroed() };
        EV_SET!(
            &mut ke,
            self.ident,
            EVFILT_USER,
            0,
            libc::NOTE_FFNOP | libc::NOTE_TRIGGER,
            0,
            ptr::null_mut()
        );

        let res = unsafe { kevent(self.kq.as_raw_fd(), &ke, 1, ptr::null_mut(), 0, ptr::null()) };

        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("kevent failed with error {}", err);
        }

        // Also write to the pipe.
        let buf = [0u8; 1];
        let res = unsafe { write(self.fds[1].as_raw_fd(), buf.as_ptr() as *const c_void, 1) };
        if res == -1 {
            // This should not happen
            let err = io::Error::last_os_error();
            panic!("write failed with error {}", err);
        }
    }
}

impl Drop for AutoResetEvent {
    fn drop(&mut self) {
        // Remove the user event from the kqueue.
        let mut ke: libc::kevent = unsafe { std::mem::zeroed() };
        EV_SET!(
            &mut ke,
            self.ident,
            EVFILT_USER,
            EV_DELETE,
            0,
            0,
            ptr::null_mut()
        );

        unsafe {
            kevent(self.kq.as_raw_fd(), &ke, 1, ptr::null_mut(), 0, ptr::null());
            // OwnedFd fields will be closed automatically
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


// It is safe to send an autoreset event to another thread. The underlying kqueue is a kernel
// object that can be used from any thread.
unsafe impl Send for AutoResetEvent {}

// It is safe to share an autoreset event between threads. The underlying kqueue is a kernel
// object that is thread-safe.
unsafe impl Sync for AutoResetEvent {}
