//! 实际控制 HTTPS 与隐藏 HWND owner 联动；无可见 UI，无在线服务。
use super::*;
use crate::desktop::composition::relay::session::Session;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
pub(super) struct Apartment;
impl Apartment {
    pub(super) fn new() -> windows::core::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
use windows::Win32::System::Threading::WaitForMultipleObjects;
pub(super) fn wait_session(session: &Session) {
    let handles: Vec<_> = session
        .wait_handles()
        .map(|h| HANDLE(h.as_raw_handle()))
        .collect();
    assert!(!handles.is_empty());
    let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
    assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
}
#[test]
fn ready_creates_hidden_window_and_close_or_revoke_stops_then_reaps_once() {
    let _apartment = Apartment::new().unwrap();
    for revoke in [false, true] {
        let (listener, client, tls) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let (poll_tx, poll_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            joined(&listener, &tls);
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.contains("after=7&timeoutMs=750"));
            poll_tx.send(()).unwrap();
            let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
        });
        let runtime = Runtime::start(client, lease, target(), |_| Ok(())).unwrap();
        let mut session = Session::from_runtime(runtime);
        wait_session(&session);
        assert!(session.poll().is_none());
        assert!(session.has_window());
        assert_eq!(session.wait_handles().count(), 2);
        poll_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        if revoke {
            drop(grant);
        } else {
            session.close_window_for_test();
        }
        let outcome = loop {
            if let Some(outcome) = session.poll() {
                break outcome;
            }
            wait_session(&session);
        };
        assert_eq!(outcome.exit, Exit::Stopped);
        assert!(outcome.window_error.is_none());
        assert!(!session.has_window());
        assert_eq!(session.wait_handles().count(), 0);
        assert!(session.poll().is_none());
        assert!(session.show().is_err());
        let _ = finish_tx.send(());
        peer.join().unwrap();
    }
}
#[test]
fn stop_before_ready_does_not_create_window_and_completion_is_single_delivery() {
    let (_listener, client, _) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    lease.revoke();
    let runtime = Runtime::start(client, lease, target(), |_| panic!("revoked consumer")).unwrap();
    let mut session = Session::from_runtime(runtime);
    session.stop();
    assert!(session.show().is_err());
    wait_session(&session);
    assert_eq!(session.poll().unwrap().exit, Exit::Stopped);
    assert!(session.poll().is_none());
    assert!(!session.has_window());
    assert_eq!(session.wait_handles().count(), 0);
}
