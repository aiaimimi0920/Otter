use super::*;
use crate::live_recovery_handshake::*;
use std::time::Duration;
fn deadline() -> Option<Instant> {
    Some(Instant::now() + Duration::from_secs(2))
}

#[test]
fn actual_overlapped_pipe_exchanges_hello_ready_without_worker_threads() {
    let (mut parent, mut child) = Pipe::pair().unwrap();
    write_hello(&mut parent.io(deadline(), None), [7; 16]).unwrap();
    let session = read_hello(&mut child.io(deadline(), None)).unwrap();
    write_ready(&mut child.io(deadline(), None), session).unwrap();
    read_ready(&mut parent.io(deadline(), None), [7; 16]).unwrap();
}
#[test]
fn timeout_cancels_pending_read_before_buffer_reuse_and_pipe_remains_usable() {
    let (mut parent, mut child) = Pipe::pair().unwrap();
    let mut buffer = [0u8; 4];
    let error = child
        .io(Some(Instant::now() + Duration::from_millis(20)), None)
        .read(&mut buffer)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    parent.io(deadline(), None).write_all(b"safe").unwrap();
    child.io(deadline(), None).read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"safe");
}
#[test]
fn closed_peer_fails_without_detached_reader_or_unbounded_wait() {
    let (mut parent, child) = Pipe::pair().unwrap();
    drop(child);
    assert!(parent.io(deadline(), None).read(&mut [0u8; 1]).is_err());
}
#[test]
fn unread_peer_causes_bounded_write_timeout_and_safe_buffer_release() {
    let (mut parent, _child) = Pipe::pair().unwrap();
    let buffer = vec![0xabu8; crate::live_recovery_wire::MAX_FRAME_BYTES];
    let mut channel = parent.io(deadline(), None);
    // The peer never consumes data; a finite number of writes fills the system pipe quota without creating an unbounded queue.
    let mut timed_out = false;
    for _ in 0..256 {
        match channel.write_all(&buffer) {
            Ok(()) => {}
            Err(error) => {
                assert_eq!(error.kind(), io::ErrorKind::TimedOut);
                timed_out = true;
                break;
            }
        }
    }
    assert!(timed_out, "Unread pipe must eventually apply backpressure");
    // Cancellation is complete and the buffer can be released; do not assume partial writes roll back or reuse this protocol stream.
    drop(buffer);
}
#[test]
fn peer_completion_handle_wakes_infinite_io_wait() {
    let (mut parent, _child) = Pipe::pair().unwrap();
    let peer = std::thread::spawn(|| {});
    // Threads and processes both have waitable handles; this test need not launch/terminate any external application.
    let error = parent
        .io(None, Some(peer.as_handle()))
        .read(&mut [0u8; 1])
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    peer.join().unwrap();
}
