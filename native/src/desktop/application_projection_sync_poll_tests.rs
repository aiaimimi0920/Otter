//! 慢收件查询与暂停不得饿死/撤销已保存关联；沿真实 TLS、750ms 调度与原位发布。
use super::super::super::poll::pause;
use super::*;

#[test]
fn projection_sync_continues_while_inbox_participation_is_paused() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let journal = fixture.journal();
        pause(&mut fixture, true);
        let rx = sync_peer(&mut fixture, vec![1]);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(binding(&fixture)["revision"], 6);
        assert_eq!(fixture.journal(), journal);
        assert!(fixture.unit_present());
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}

#[test]
fn projection_sync_gets_real_slot_after_slow_inbox_completion() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let listener = fixture.listener.take().unwrap();
        let tls = fixture.tls.clone();
        let peer_started = Instant::now();
        fixture.server = Some(std::thread::spawn(move || {
            eprintln!(
                "projection-slow-inbox-peer: wait elapsed_ms={}",
                peer_started.elapsed().as_millis()
            );
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            eprintln!(
                "projection-slow-inbox-peer: request elapsed_ms={}",
                peer_started.elapsed().as_millis()
            );
            assert!(head.starts_with("POST /v1/projections/inbox "));
            assert_eq!(body, serde_json::json!({"policy":"confirm"}));
            // 本用例故意慢响应：连接/握手后仅为这一逻辑响应设置独立 5 秒预算，不逐片续期。
            stream
                .sock
                .operation_deadline(Duration::from_secs(5))
                .unwrap();
            std::thread::sleep(Duration::from_millis(3100));
            eprintln!(
                "projection-slow-inbox-peer: reply elapsed_ms={}",
                peer_started.elapsed().as_millis()
            );
            reply(&mut stream, 200, br#"{"invitations":[]}"#);
            listener
        }));
        fixture.app.resume_projection_poll_for_test();
        let started = Instant::now();
        fixture.app.process_projection_automatic(
            &mut |origin| config(origin, fixture.files.root()),
            started,
        );
        // 新慢服务用例独立覆盖连接准备+3.1秒延迟，原通用 wait_remote 的 5 秒门不变。
        let handle = fixture.app.remote_sessions.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 10_000),
            WAIT_OBJECT_0
        );
        joined(&mut fixture);
        assert!(started.elapsed() >= Duration::from_secs(3));
        fixture.app.poll_projection_inbox();
        fixture.app.process_projection_automatic(
            &mut |_| panic!("slow completion immediately repolled inbox"),
            Instant::now(),
        );
        assert!(!fixture.app.remote_sessions.projection_pending());
        let rx = sync_peer(&mut fixture, vec![1]);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(binding(&fixture)["revision"], 6);
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}
