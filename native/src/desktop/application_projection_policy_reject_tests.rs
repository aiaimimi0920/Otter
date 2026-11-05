//! 自动拒绝沿真实查询/receipt，服务端仍收到 confirm；不把本地拒绝冒充 disabled。
use super::*;
use crate::{desktop::workspace_profile::Profile, projection_receive_policy::Policy};

#[test]
fn receive_policy_reject_automatically_queries_and_rejects_over_original_tls_single_slot() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let server_origin = origin.clone();
    let (tx, _rx) = mpsc::sync_channel(1);
    let server = std::thread::spawn(move || peer::serve(listener, tls, server_origin, 0, tx));
    unsafe {
        let (_class, mut app) = application();
        app.install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), false)
            .unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        while !matches!(
            app.projection_profile,
            crate::desktop::application::projection::profile::Owner::Ready { .. }
        ) {
            app.process_authorization();
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        app.authorization.borrow_mut().request = Some(Request::ReceivePolicy(Policy::Reject));
        app.process_projection_policy();
        while app.authorization.borrow().consent.saving {
            app.process_projection_policy();
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        assert_eq!(app.authorization.borrow().consent.policy(), Policy::Reject);
        app.authorization.borrow_mut().origin = Some(origin);
        app.authorization_owner
            .start(Task::Renew {
                client,
                store: identity,
            })
            .unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        let started = Instant::now();
        let mut previous = None;
        let mut advanced = false;
        loop {
            app.process_authorization_with_projection(|origin| config(origin, files.root()));
            let view = app.authorization.borrow();
            let stage = (
                view.busy,
                view.projection.busy,
                view.projection.entries.len(),
                view.projection.status == "已拒绝投送",
            );
            if previous != Some(stage) {
                eprintln!(
                    "projection-auto-reject: stage={stage:?} elapsed_ms={}",
                    started.elapsed().as_millis()
                );
                previous = Some(stage);
            }
            drop(view);
            if !advanced
                && !app.authorization.borrow().projection.busy
                && !app.authorization.borrow().projection.entries.is_empty()
            {
                // 对端原 5 秒预算不含 3 秒调度等待再加 IPv6 fallback；只推进调度输入，不改墙钟。
                let due = app.projection_policy_deadline().unwrap();
                assert!(due > Instant::now());
                app.process_projection_automatic(
                    &mut |_| panic!("automatic rejection started before deadline"),
                    due - Duration::from_nanos(1),
                );
                assert!(!app.remote_sessions.projection_pending());
                app.process_projection_automatic(&mut |origin| config(origin, files.root()), due);
                advanced = true;
            }
            if app.authorization.borrow().projection.status == "已拒绝投送" {
                break;
            }
            assert!(Instant::now() < end, "automatic rejection deadline");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(advanced);
        assert!(app.authorization.borrow().projection.entries.is_empty());
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        assert_eq!(app.documents.len(), 1);
        app.shutdown();
    }
    let listener = server.join().unwrap();
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        before
    );
}
