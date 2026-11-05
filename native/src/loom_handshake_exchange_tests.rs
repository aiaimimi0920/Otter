//! 真实 TLS 能力交换及取消，不读取发现文件或用户凭据。
use super::*;
use crate::loom_handshake_exchange::{fetch, Error};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
#[test]
fn catalog_worker_notifies_once_and_rejects_cancelled_completed_result() {
    use crate::desktop::capability_job::{Error as JobError, Job, Source};
    for cancelled in [false, true] {
        let (listener, identity, config) = fixture();
        let server = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            socket.read().unwrap();
            socket
                .send(tungstenite::Message::Text(
                    crate::loom_handshake::tests::fixture().to_string(),
                ))
                .unwrap();
        });
        let mut job = Job::default();
        let (send, receive) = std::sync::mpsc::channel();
        job.start(
            Source::Fixed(identity),
            Arc::new(move || {
                send.send(()).unwrap();
            }),
        )
        .unwrap();
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(job.is_busy());
        if cancelled {
            job.cancel();
        }
        let result = job.poll().expect("notification publishes completion");
        if cancelled {
            assert!(matches!(result, Err(JobError::Cancelled)));
        } else {
            assert_eq!(result.unwrap().handshake.arts().len(), 1);
        }
        assert!(!job.is_busy());
        assert!(job.poll().is_none());
        server.join().unwrap();
    }
}

#[test]
fn authenticated_catalog_exchange_ignores_events_and_handles_ping() {
    let (listener, identity, config) = fixture();
    let server = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        let wire: serde_json::Value =
            serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
        assert_eq!(wire["method"], "loom.hook.handshake");
        assert_eq!(wire["params"]["platform"], "windows");
        socket
            .send(tungstenite::Message::Text(
                serde_json::json!({"method":"loom.hook.capabilities.updated"}).to_string(),
            ))
            .unwrap();
        socket.send(tungstenite::Message::Ping(vec![1, 2])).unwrap();
        assert!(
            matches!(socket.read().unwrap(), tungstenite::Message::Pong(value) if value == [1, 2])
        );
        socket
            .send(tungstenite::Message::Text(
                crate::loom_handshake::tests::fixture().to_string(),
            ))
            .unwrap();
    });
    let execution_identity = identity.clone();
    drop(identity);
    let result = fetch(&execution_identity, &AtomicBool::new(false)).unwrap();
    assert_eq!(result.arts()[0]["id"], "test-art");
    server.join().unwrap();
}
#[test]
fn cancellation_before_connect_and_during_response_wait_is_observed() {
    let (listener, identity, config) = fixture();
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        fetch(&identity, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    listener.set_nonblocking(false).unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        socket.read().unwrap();
        send.send(()).unwrap();
        assert!(socket.read().is_err());
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let worker = thread::spawn(move || fetch(&identity, &signal));
    receive.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    cancel.store(true, Ordering::Release);
    assert!(matches!(worker.join().unwrap(), Err(Error::Cancelled)));
    assert!(start.elapsed() < Duration::from_secs(2));
    server.join().unwrap();
}
