//! 用认证回执的等待点控制媒体自然掉线与控制轮换的交错，不依靠 sleep 命中窗口。
use super::*;

#[test]
fn source_old_progress_cannot_acknowledge_concurrent_media_rotation() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (held, awaiting_rotation) = mpsc::channel();
    let (rotated, send_old_watermark) = mpsc::channel();
    let (replacement, awaiting_watermark) = mpsc::channel();
    let (allow, send_new_watermark) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut value = created(&listener, &tls);
        let mut nonces = std::collections::HashSet::new();
        let mut initial = socket(&listener, &tls, &mut nonces);
        frame(&mut initial, 1);
        drop(initial);
        let mut old = socket(&listener, &tls, &mut nonces);
        frame(&mut old, 2);
        let receipt = || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/sessions/") && !head.contains("/events"));
            stream
        };
        let mut old_receipt = receipt();
        drop(old);
        held.send(()).unwrap();
        send_old_watermark
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        value["sourceConnected"] = json!(true);
        value["lastFrameId"] = json!(2);
        reply(&mut old_receipt, 200, &serde_json::to_vec(&value).unwrap());
        // 尚未消费的控制侧请求必须由确已升级的连接完成，且该连接不得发送帧。
        let mut retired = socket(&listener, &tls, &mut nonces);
        assert!(retired.read().is_err(), "pending rotation sent a frame");
        let mut current = socket(&listener, &tls, &mut nonces);
        frame(&mut current, 3);
        let mut new_receipt = receipt();
        replacement.send(()).unwrap();
        send_new_watermark
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        value["lastFrameId"] = json!(3);
        reply(&mut new_receipt, 200, &serde_json::to_vec(&value).unwrap());
        assert_eq!(nonces.len(), 4);
        closed(&listener, &tls, &mut value);
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    awaiting_rotation
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    assert!(runtime.recovering());
    runtime.test_request_media_restart();
    rotated.send(()).unwrap();
    awaiting_watermark
        .recv_timeout(Duration::from_secs(8))
        .unwrap();
    assert!(runtime.recovering());
    assert!(!runtime.ready());
    allow.send(()).unwrap();
    spin(|| runtime.ready());
    runtime.request_stop();
    spin(|| runtime.test_finished());
    let outcome = runtime.try_join().unwrap();
    assert_eq!(outcome.exit, Exit::Stopped);
    assert_eq!(outcome.cleanup, Cleanup::Confirmed);
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    peer.join().unwrap();
}

#[test]
fn slow_source_progress_polls_share_the_same_confirmation_deadline() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (confirmed, wait) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut value = created(&listener, &tls);
        let mut nonces = std::collections::HashSet::new();
        let mut first = socket(&listener, &tls, &mut nonces);
        frame(&mut first, 1);
        drop(first);
        let mut second = socket(&listener, &tls, &mut nonces);
        frame(&mut second, 2);
        let receipt = || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/sessions/") && !head.contains("/events"));
            stream
        };
        let mut lagging = receipt();
        let began = Instant::now();
        std::thread::sleep(Duration::from_secs(2));
        value["sourceConnected"] = json!(true);
        value["lastFrameId"] = json!(1);
        reply(&mut lagging, 200, &serde_json::to_vec(&value).unwrap());
        drop(lagging);
        let mut withheld = receipt();
        assert!(withheld.read_exact(&mut [0]).is_err());
        // 给调度留余量，但不允许第二次 GET 从头获得三秒（旧实现需超过五秒）。
        assert!(
            began.elapsed() < Duration::from_millis(4500),
            "confirmation deadline restarted: {:?}",
            began.elapsed()
        );
        drop(second);
        let mut third = socket(&listener, &tls, &mut nonces);
        frame(&mut third, 3);
        value["lastFrameId"] = json!(3);
        reply(&mut receipt(), 200, &serde_json::to_vec(&value).unwrap());
        assert_eq!(nonces.len(), 3);
        confirmed.send(()).unwrap();
        closed(&listener, &tls, &mut value);
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    spin(|| runtime.recovering());
    assert!(!runtime.ready());
    wait.recv_timeout(Duration::from_secs(9)).unwrap();
    spin(|| runtime.ready());
    runtime.request_stop();
    spin(|| runtime.test_finished());
    let outcome = runtime.try_join().unwrap();
    assert_eq!(outcome.exit, Exit::Stopped);
    assert_eq!(outcome.cleanup, Cleanup::Confirmed);
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    peer.join().unwrap();
}
