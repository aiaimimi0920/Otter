//! 仅使用自建回环 socket，不读取发现凭据或接触运行中的 Loom。
use super::*;
use std::{net::TcpListener, sync::mpsc, thread};

fn pair() -> (BridgeStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client =
        TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
            .unwrap();
    let server = listener.accept().unwrap().0;
    server
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    server
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    (
        BridgeStream::new(client, Instant::now() + Duration::from_secs(2)).unwrap(),
        server,
    )
}

#[test]
fn fragmented_io_does_not_refresh_deadline() {
    let (mut stream, mut server) = pair();
    let original = stream.deadline;
    server.write_all(b"ab").unwrap();
    let mut byte = [0];
    stream.read_exact(&mut byte).unwrap();
    assert_eq!(byte, *b"a");
    stream.read_exact(&mut byte).unwrap();
    assert_eq!(byte, *b"b");
    stream.write_all(b"reply").unwrap();
    stream.flush().unwrap();
    let mut reply = [0; 5];
    server.read_exact(&mut reply).unwrap();
    assert_eq!(&reply, b"reply");
    assert_eq!(stream.deadline, original);
}

#[test]
fn expired_operations_fail_before_consuming_buffered_data_and_can_be_renewed() {
    let (mut stream, mut server) = pair();
    server.write_all(b"x").unwrap();
    stream.deadline = Instant::now();
    assert_eq!(
        stream.read(&mut [0]).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(
        stream.write(b"no").unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(stream.flush().unwrap_err().kind(), io::ErrorKind::TimedOut);
    stream.operation_deadline(Duration::from_secs(2)).unwrap();
    let mut byte = [0];
    stream.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"x");
}

#[test]
fn cancellation_interrupts_read_without_waiting_for_long_deadline() {
    let (mut stream, _server) = pair();
    stream.operation_deadline(Duration::from_secs(30)).unwrap();
    let interrupt = stream.interrupt_handle().unwrap();
    #[cfg(windows)]
    let entered = stream.wait.entered.clone();
    let (send, receive) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = stream.read(&mut [0]);
        let _ = send.send(result);
    });
    #[cfg(windows)]
    {
        let deadline = Instant::now() + Duration::from_secs(2);
        while entered.load(Ordering::Acquire) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
    }
    interrupt.cancel().unwrap();
    let result = receive.recv_timeout(Duration::from_secs(3));
    // 失败路径也先回收 worker；不能在 expect panic 后留下最长 30 秒的后台读取。
    interrupt.cancel().unwrap();
    worker.join().unwrap();
    let result = result.expect("cancel must wake read");
    #[cfg(windows)]
    assert!(
        entered.load(Ordering::Acquire) > 0,
        "test must reach WouldBlock event wait"
    );
    assert!(!matches!(result, Ok(count) if count > 0));
    interrupt.cancel().unwrap();
}

#[cfg(windows)]
#[path = "loom_bridge_stream_windows_tests.rs"]
mod windows_tests;

#[test]
fn cancellation_before_worker_read_is_not_lost() {
    let (mut stream, _server) = pair();
    let interrupt = stream.interrupt_handle().unwrap();
    let (start, ready) = mpsc::channel();
    let worker = thread::spawn(move || {
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        stream.read(&mut [0])
    });
    interrupt.cancel().unwrap();
    start.send(()).unwrap();
    let result = worker.join().unwrap();
    assert!(!matches!(result, Ok(count) if count > 0));
}

#[test]
fn cancellation_after_completed_read_prevents_next_read() {
    let (mut stream, mut server) = pair();
    let interrupt = stream.interrupt_handle().unwrap();
    server.write_all(b"x").unwrap();
    let mut byte = [0];
    stream.read_exact(&mut byte).unwrap();
    assert_eq!(byte, *b"x");
    interrupt.cancel().unwrap();
    let started = Instant::now();
    assert!(!matches!(stream.read(&mut byte), Ok(count) if count > 0));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn cancellation_is_shared_permanent_and_not_retried_by_exact_io() {
    let (mut stream, mut server) = pair();
    let first = stream.interrupt_handle().unwrap();
    let second = stream.interrupt_handle().unwrap();
    server.write_all(b"already buffered").unwrap();
    first.cancel().unwrap();
    assert_eq!(
        stream.read_exact(&mut [0]).unwrap_err().kind(),
        io::ErrorKind::ConnectionAborted
    );
    assert_eq!(
        stream.write_all(b"must not send").unwrap_err().kind(),
        io::ErrorKind::ConnectionAborted
    );
    assert_eq!(
        stream.flush().unwrap_err().kind(),
        io::ErrorKind::ConnectionAborted
    );
    assert_eq!(
        stream
            .operation_deadline(Duration::from_secs(30))
            .unwrap_err()
            .kind(),
        io::ErrorKind::ConnectionAborted
    );
    second.cancel().unwrap();
    assert_eq!(
        stream.read(&mut [0]).unwrap_err().kind(),
        io::ErrorKind::ConnectionAborted
    );
}

#[test]
fn stalled_peer_is_bounded_by_socket_deadline() {
    let (mut stream, _server) = pair();
    stream
        .operation_deadline(Duration::from_millis(50))
        .unwrap();
    let started = Instant::now();
    let error = stream.read(&mut [0]).unwrap_err();
    assert!(matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn invalid_duration_does_not_change_existing_deadline() {
    let (mut stream, _server) = pair();
    let original = stream.deadline;
    for duration in [Duration::ZERO, Duration::MAX] {
        assert_eq!(
            stream.operation_deadline(duration).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(stream.deadline, original);
    }
}
