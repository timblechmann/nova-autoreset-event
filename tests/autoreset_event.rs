use std::sync::Arc;
use std::thread;
use std::time::Duration;

use nova_autoreset_event::AutoResetEvent;

#[test]
fn test_autoreset_event() {
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

#[test]
fn test_wait_does_not_return_early() {
    let event = Arc::new(AutoResetEvent::new().unwrap());
    let event2 = event.clone();

    let start = std::time::Instant::now();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        event2.signal();
    });

    event.wait();
    let elapsed = start.elapsed();
    // Allow for a small margin of error, but it should definitely be close to 100ms
    assert!(
        elapsed >= Duration::from_millis(90),
        "Wait returned too early: {:?}",
        elapsed
    );
}

#[test]
fn test_try_wait() {
    let event = AutoResetEvent::new().unwrap();
    assert!(!event.try_wait());

    event.signal();
    assert!(event.try_wait());
    assert!(!event.try_wait());
}

#[test]
fn test_try_wait_for() {
    let event = Arc::new(AutoResetEvent::new().unwrap());
    assert!(!event.try_wait_for(Duration::from_millis(10)));

    let event2 = event.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        event2.signal();
    });

    assert!(event.try_wait_for(Duration::from_millis(1000)));
    assert!(!event.try_wait());
}

#[cfg(unix)]
#[tokio::test]
async fn test_tokio() {
    use std::os::unix::io::AsRawFd;

    let event = Arc::new(AutoResetEvent::new().unwrap());

    // Create AsyncFd once - it should be reused for multiple signals
    let async_fd = tokio::io::unix::AsyncFd::new(event.as_raw_fd()).unwrap();

    // Test multiple signals
    for i in 0..3 {
        assert!(
            !event.try_wait(),
            "Event should not be signaled at start of iteration {}",
            i
        );

        let event_clone = event.clone();

        let thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            event_clone.signal();
        });

        // Wait for readability (this just tells us the event was signaled)
        let mut guard = async_fd.readable().await.unwrap();

        // Use wait() to properly consume the signal
        // wait() will not block because we know the event is signaled
        event.wait();

        // Clear the readiness so we can wait again
        guard.clear_ready();

        thread.join().unwrap();
    }
}
