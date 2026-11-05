//! 已借出的 Control 仍使用租约专属取消 owner，缓存撤销可唤醒进行中的 HTTPS。
use super::*;
use crate::device_authorization_lease::Grant;

#[test]
fn cache_revocation_cancels_inflight_control_even_after_control_was_borrowed() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let mut grant = Grant::new(session);
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("GET /v1/live/sessions/live:test%2Fpart HTTP/1.1"));
        ready_tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
    });
    let worker = std::thread::spawn(move || {
        let control = lease.control(&client).unwrap();
        control.snapshot("live:test/part").is_err()
    });
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let start = std::time::Instant::now();
    drop(grant);
    let failed = worker.join().unwrap();
    let elapsed = start.elapsed();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(failed);
    assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
}
