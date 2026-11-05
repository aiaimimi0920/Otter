//! 首次批准等待的真实 HTTPS 验证，复用父模块的限时本机 TLS 服务。
use super::*;
use crate::{device_approval, device_request_cancel::Cancel};

fn identity() -> Identity {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device-1").unwrap();
    identity
}

#[test]
fn pending_then_approved_uses_new_challenge_without_registering_again() {
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        let start = Instant::now();
        for index in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            if index < 2 {
                assert!(head.starts_with("POST /v1/device-sessions/challenges HTTP/1.1"));
            } else {
                assert!(head.starts_with("POST /v1/device-sessions HTTP/1.1"));
            }
            match index {
                0 => reply(&mut stream, 403, br#"{"error":{"code":"device_not_authorized"}}"#),
                1 => {
                    assert!(start.elapsed() >= Duration::from_millis(250));
                    reply(&mut stream, 200, br#"{"deviceId":"device-1","challengeId":"id","challenge":"value"}"#);
                }
                _ => reply(&mut stream, 200, br#"{"deviceId":"device-1","token":"approved","expiresAtMs":18446744073709551615}"#),
            }
        }
    });
    let session = device_approval::wait(&client, &identity(), &Cancel::new()).unwrap();
    assert_eq!(session.token_for(&client.origin, 0), Ok("approved"));
    peer.join().unwrap();
}

#[test]
fn fatal_challenge_does_not_poll_and_renewal_pending_remains_single_attempt() {
    for renewal in [false, true] {
        let (listener, client, config) = fixture();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            let body = if renewal {
                br#"{"error":{"code":"device_not_authorized"}}"#.as_slice()
            } else {
                br#"{"error":{"code":"device_revoked"}}"#
            };
            reply(&mut stream, 403, body);
            assert!(stream.read(&mut [0]).map_or(true, |n| n == 0));
            assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        });
        if renewal {
            assert_eq!(
                client
                    .issue_once(&identity(), &AtomicBool::new(false), |_| {})
                    .err(),
                Some(Error::Protocol(
                    crate::device_session_attempt::Error::PendingApproval
                ))
            );
        } else {
            assert_eq!(
                device_approval::wait(&client, &identity(), &Cancel::new()).err(),
                Some(device_approval::Error::Request(Error::Protocol(
                    crate::device_session_attempt::Error::Http(403)
                )))
            );
        }
        peer.join().unwrap();
    }
}

#[test]
fn approval_total_deadline_bounds_an_inflight_request() {
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        // 不发送响应；客户端总期限必须小于普通 POST 的十秒期限。
        assert!(stream.read(&mut [0]).map_or(true, |n| n == 0));
    });
    let start = Instant::now();
    assert_eq!(
        device_approval::wait_until(
            &client,
            &identity(),
            &Cancel::new(),
            start + Duration::from_millis(300)
        )
        .err(),
        Some(device_approval::Error::Timeout)
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    peer.join().unwrap();
}

#[test]
fn cancellation_interrupts_inflight_approval_and_leaves_no_retry() {
    let (listener, client, config) = fixture();
    let cancel = Arc::new(Cancel::new());
    let worker_cancel = cancel.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        ready_tx.send(()).unwrap();
        assert!(stream.read(&mut [0]).map_or(true, |n| n == 0));
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    });
    let worker = std::thread::spawn(move || {
        device_approval::wait(&client, &identity(), &worker_cancel).err()
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    cancel.cancel();
    assert_eq!(
        worker.join().unwrap(),
        Some(device_approval::Error::Stopped)
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    peer.join().unwrap();
}
