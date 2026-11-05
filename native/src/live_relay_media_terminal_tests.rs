//! 媒体终态必须唤醒在途控制请求，而非等待其三秒期限并掩盖真实原因。
use super::*;
use crate::{byte_budget::Budget, live_relay_viewer_runtime::Exit as MediaExit};

#[test]
fn media_close_preserves_reason_and_interrupts_inflight_control_poll() {
    let _apartment = super::desktop_session::Apartment::new().unwrap();
    for revoked in [false, true] {
        let (listener, client, tls) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let (inflight_tx, inflight_rx) = std::sync::mpsc::channel();
        let inflight = Arc::new(std::sync::Mutex::new(inflight_rx));
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let finish = Arc::new(std::sync::Mutex::new(finish_rx));
        let (closed_tx, closed_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            joined(&listener, &tls);
            let mut children = Vec::new();
            for _ in 0..2 {
                let stream = accept(&listener);
                let tls = tls.clone();
                let (inflight, inflight_tx, finish, closed_tx) = (
                    inflight.clone(),
                    inflight_tx.clone(),
                    finish.clone(),
                    closed_tx.clone(),
                );
                children.push(std::thread::spawn(move || {
                    let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), stream);
                    let (head, _) = request(&mut stream);
                    if head.starts_with("GET /v1/live/media?") {
                        let mut socket = super::playback::upgrade_media(stream, &head);
                        inflight
                            .lock()
                            .unwrap()
                            .recv_timeout(std::time::Duration::from_secs(5))
                            .unwrap();
                        use tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
                        closed_tx.send(std::time::Instant::now()).unwrap();
                        socket
                            .close(Some(CloseFrame {
                                code: if revoked {
                                    CloseCode::Policy
                                } else {
                                    CloseCode::Normal
                                },
                                reason: if revoked {
                                    "live_media_device_revoked"
                                } else {
                                    "closed"
                                }
                                .into(),
                            }))
                            .unwrap();
                    } else {
                        assert!(head.contains("after=7&timeoutMs=750"));
                        inflight_tx.send(()).unwrap();
                        let _ = finish
                            .lock()
                            .unwrap()
                            .recv_timeout(std::time::Duration::from_secs(8));
                    }
                }));
            }
            for child in children {
                child.join().unwrap();
            }
        });
        let runtime = Runtime::start_viewer(
            client,
            lease,
            target(),
            Budget::new(1024 * 1024),
            |_| Ok(()),
        )
        .unwrap();
        let mut session =
            crate::desktop::composition::relay::session::Session::from_runtime(runtime);
        let sent = closed_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let outcome = loop {
            if let Some(outcome) = session.poll() {
                break outcome;
            }
            super::desktop_session::wait_session(&session);
        };
        let elapsed = sent.elapsed();
        let result = Some(outcome.exit);
        assert!(outcome.window_error.is_none());
        let _ = finish_tx.send(());
        peer.join().unwrap();
        assert_eq!(
            result,
            Some(Exit::Media(if revoked {
                MediaExit::DeviceRevoked
            } else {
                MediaExit::Closed
            }))
        );
        assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
        assert!(!session.has_window());
        assert_eq!(session.wait_handles().count(), 0);
        assert!(session.poll().is_none());
    }
}
