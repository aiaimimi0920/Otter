//! 控制线程的实际 HTTPS 加入/重放/轮询与 Win32 就绪、完成句柄合同。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    live_relay_control_owner::Error as OwnerError,
    live_relay_control_runtime::{Exit, Runtime, Target},
};
use std::os::windows::io::{AsRawHandle, BorrowedHandle};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
#[path = "live_relay_control_configured_tests.rs"]
mod configured;
#[path = "live_relay_desktop_session_tests.rs"]
mod desktop_session;
#[path = "live_relay_viewer_reconnect_http_tests.rs"]
mod media_reconnect;
#[path = "live_relay_media_terminal_tests.rs"]
mod media_terminal;
#[path = "live_relay_playback_tests.rs"]
mod playback;
#[path = "live_relay_control_reconnect_http_tests.rs"]
mod read_reconnect;

fn wait(handle: BorrowedHandle<'_>) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn target() -> Target {
    Target {
        session: "live:test/part".into(),
        surface: "surface-1".into(),
        attachment: "attachment-1".into(),
        expected: None,
        selected: None,
    }
}
fn joined(listener: &TcpListener, config: &Arc<ServerConfig>) {
    for index in 0..4 {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(listener),
        );
        let (head, _) = request(&mut stream);
        match index {
            0 => {
                assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
                let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                value["session"]["viewerDevices"] = serde_json::json!([]);
                reply(
                    &mut stream,
                    200,
                    &serde_json::to_vec(&serde_json::json!({
                    "protocolVersion":"loom.live.v1","sessions":[value]}))
                    .unwrap(),
                );
            }
            1 => {
                assert!(head.starts_with("POST "));
                reply(&mut stream, 200, &snapshot());
            }
            2 => {
                assert!(head.contains("after=0&timeoutMs=0"));
                reply(&mut stream, 200, &history(true));
            }
            _ => {
                assert!(head.contains("after=6&timeoutMs=750"));
                reply(&mut stream, 200, &history(false));
            }
        }
    }
}
#[test]
fn ready_then_cache_revoke_interrupts_poll_and_reaps_control_thread() {
    for take_ready in [true, false] {
        let (listener, client, config) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let (poll_tx, poll_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            joined(&listener, &config);
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.contains("after=7&timeoutMs=750"));
            poll_tx.send(()).unwrap();
            let _ = done_rx.recv_timeout(std::time::Duration::from_secs(8));
        });
        let ui = std::thread::current().id();
        let mut runtime = Runtime::start(client, lease, target(), move |events| {
            assert_ne!(std::thread::current().id(), ui);
            assert_eq!(events.len(), 1);
            Ok(())
        })
        .unwrap();
        wait(runtime.ready_handle());
        let ready = take_ready.then(|| runtime.try_ready().unwrap());
        if let Some(ready) = &ready {
            assert_eq!(ready.baseline.control, 1);
            assert!(ready.media.validate(&ready.permit).is_ok());
        }
        poll_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        if take_ready {
            drop(grant);
        } else {
            runtime.request_stop();
        }
        if let Some(ready) = &ready {
            assert!(ready.media.validate(&ready.permit).is_err());
        }
        wait(runtime.wait_handle().unwrap());
        assert_eq!(runtime.try_join(), Some(Exit::Stopped));
        assert!(runtime.wait_handle().is_none());
        assert!(runtime.try_ready().is_none());
        let _ = done_tx.send(());
        peer.join().unwrap();
    }
}
#[test]
fn consumer_failure_or_panic_never_publishes_ready() {
    for panic in [false, true] {
        let (listener, client, config) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let peer = std::thread::spawn(move || joined(&listener, &config));
        let mut runtime = Runtime::start(client, lease, target(), move |_| {
            assert!(!panic, "consumer fixture panic");
            Err(())
        })
        .unwrap();
        wait(runtime.wait_handle().unwrap());
        assert_eq!(
            runtime.try_join(),
            Some(if panic {
                Exit::Panicked
            } else {
                Exit::Control(OwnerError::Consumer)
            })
        );
        assert!(runtime.try_ready().is_none());
        peer.join().unwrap();
    }
}
#[test]
fn already_revoked_lease_finishes_without_network_or_ready() {
    let (_listener, client, _) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    drop(grant);
    let mut runtime =
        Runtime::start(client, lease, target(), |_| panic!("revoked consumer")).unwrap();
    wait(runtime.wait_handle().unwrap());
    assert_eq!(runtime.try_join(), Some(Exit::Stopped));
    assert!(runtime.try_ready().is_none());
}

#[test]
fn completed_unreaped_threads_keep_capacity_until_joined() {
    let (_listener, client, _) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let mut runtimes = Vec::new();
    for _ in 0..4 {
        let lease = grant.lease(&client.origin, 0).unwrap();
        lease.revoke();
        let runtime = Runtime::start(client.clone(), lease, target(), |_| Ok(())).unwrap();
        wait(runtime.wait_handle().unwrap());
        runtimes.push(runtime);
    }
    let lease = grant.lease(&client.origin, 0).unwrap();
    assert!(
        matches!(Runtime::start(client.clone(), lease, target(), |_| Ok(())),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert_eq!(runtimes[0].try_join(), Some(Exit::Stopped));
    let lease = grant.lease(&client.origin, 0).unwrap();
    lease.revoke();
    let mut replacement = Runtime::start(client, lease, target(), |_| Ok(())).unwrap();
    wait(replacement.wait_handle().unwrap());
    assert_eq!(replacement.try_join(), Some(Exit::Stopped));
    for runtime in &mut runtimes[1..] {
        assert_eq!(runtime.try_join(), Some(Exit::Stopped));
    }
}
