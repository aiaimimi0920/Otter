//! 真实本机 TLS/WSS 验证授权拒绝是终态，不等待错误正文或继续发请求。
use super::*;

#[test]
fn source_progress_denied_without_body_never_reconnects_or_sends_close() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (joined, done) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        created(&listener, &tls);
        let mut nonces = std::collections::HashSet::new();
        let mut first = socket(&listener, &tls, &mut nonces);
        frame(&mut first, 1);
        drop(first);
        let mut second = socket(&listener, &tls, &mut nonces);
        frame(&mut second, 2);
        let mut progress = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, _) = request(&mut progress);
        assert!(head.starts_with("GET /v1/live/sessions/") && !head.contains("/events"));
        deny_without_body(&mut progress);
        // 保持错误体未发送且连接未关；不能等三秒的 HTTP 正文截止才退出。
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    spin(|| runtime.test_finished());
    let outcome = runtime.try_join().unwrap();
    assert_eq!(
        outcome.exit,
        Exit::Http(crate::live_relay_http::Error::AuthorizationRequired)
    );
    assert_eq!(outcome.cleanup, Cleanup::AuthorizationEnded);
    assert!(!runtime.ready());
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    joined.send(()).unwrap();
    peer.join().unwrap();
}

#[test]
fn source_close_denied_after_explicit_stop_is_not_unconfirmed_or_retried() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (joined, done) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        created(&listener, &tls);
        let mut media = socket(&listener, &tls, &mut std::collections::HashSet::new());
        frame(&mut media, 1);
        let mut close = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut close);
        assert!(head.starts_with("POST ") && head.contains("/close HTTP/1.1"));
        assert_eq!(body["envelope"]["sequence"], 2);
        deny_without_body(&mut close);
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    spin(|| runtime.ready());
    runtime.request_stop();
    spin(|| runtime.test_finished());
    let outcome = runtime.try_join().unwrap();
    assert_eq!(outcome.exit, Exit::Stopped);
    assert_eq!(outcome.cleanup, Cleanup::AuthorizationEnded);
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    joined.send(()).unwrap();
    peer.join().unwrap();
}

fn deny_without_body(stream: &mut impl Write) {
    stream
        .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 123\r\nConnection: close\r\n\r\n")
        .unwrap();
    stream.flush().unwrap();
}
