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
    // 对端始终不消费；有限次数写入填满系统管道配额，不能生成无界队列。
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
    // 取消已完成，缓冲区可释放；不假定部分写入回滚，也不复用该协议流。
    drop(buffer);
}
#[test]
fn peer_completion_handle_wakes_infinite_io_wait() {
    let (mut parent, _child) = Pipe::pair().unwrap();
    let peer = std::thread::spawn(|| {});
    // 线程与进程均为可等待句柄；本测试不用启动/结束任何外部应用。
    let error = parent
        .io(None, Some(peer.as_handle()))
        .read(&mut [0u8; 1])
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    peer.join().unwrap();
}
