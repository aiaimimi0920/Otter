//! 原生就绪等待覆盖写背压、数据后 EOF，以及读写超时后的永久取消。
use super::*;
use std::os::windows::io::AsRawSocket;
use windows::Win32::Networking::WinSock::*;

fn small_send_buffer(stream: &BridgeStream) {
    let bytes = 4096i32.to_ne_bytes();
    assert_eq!(
        unsafe {
            setsockopt(
                SOCKET(stream.socket.as_raw_socket() as usize),
                SOL_SOCKET,
                SO_SNDBUF,
                Some(&bytes),
            )
        },
        0
    );
}
#[test]
fn backpressured_write_is_cancelled_by_event_not_peer_or_long_timeout() {
    let (mut stream, _server) = pair();
    small_send_buffer(&stream);
    stream.operation_deadline(Duration::from_secs(10)).unwrap();
    let entered = stream.wait.entered.clone();
    let interrupt = stream.interrupt_handle().unwrap();
    let (send, receive) = mpsc::channel();
    let worker = thread::spawn(move || {
        let bytes = [7; 65536];
        loop {
            if let Err(error) = stream.write_all(&bytes) {
                send.send(error.kind()).unwrap();
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while entered.load(Ordering::Acquire) == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    let started = Instant::now();
    interrupt.cancel().unwrap();
    let result = receive.recv_timeout(Duration::from_secs(2));
    worker.join().unwrap();
    assert!(
        entered.load(Ordering::Acquire) > 0,
        "test must reach write readiness wait"
    );
    assert_eq!(result.unwrap(), io::ErrorKind::ConnectionAborted);
    assert!(started.elapsed() < Duration::from_secs(2));
    // stream/socket 已销毁，独立取消事件仍由 Interrupt 保活，重复取消安全。
    interrupt.cancel().unwrap();
}
#[test]
fn backpressure_keeps_absolute_deadline_and_does_not_expose_would_block() {
    let (mut stream, _server) = pair();
    small_send_buffer(&stream);
    stream
        .operation_deadline(Duration::from_millis(60))
        .unwrap();
    let entered = stream.wait.entered.clone();
    let started = Instant::now();
    let bytes = [3; 65536];
    let error = loop {
        if let Err(error) = stream.write_all(&bytes) {
            break error;
        }
    };
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(entered.load(Ordering::Acquire) > 0);
    assert!(started.elapsed() < Duration::from_secs(2));
}
#[test]
fn close_notification_does_not_discard_buffered_bytes() {
    let (mut stream, mut server) = pair();
    server.write_all(b"before EOF").unwrap();
    server.shutdown(std::net::Shutdown::Write).unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"before EOF");
}
