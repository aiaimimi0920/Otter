//! Slow inbox queries and pauses must neither starve nor revoke saved associations; real TLS, 750ms scheduling, and in-place publication.
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
            // Deliberately slow response: after connection/handshake, give only this logical response an independent 5-second budget, without renewal per chunk.
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
        // The new slow-service case separately covers connection preparation plus 3.1 seconds of delay; the original shared wait_remote 5-second gate stays unchanged.
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
