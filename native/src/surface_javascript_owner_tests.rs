//! 真实 TLS→持久后台 owner；UI 不调用 refresh，闲置撤权/到期也必须回收 child。
use super::*;
use crate::surface_javascript::authorized::Runtime;
#[path = "surface_javascript_interaction_tests.rs"]
mod interaction;
fn copy(mount: &MountedSurface) -> MountedSurface {
    MountedSurface {
        binding: mount.binding.clone(),
        snapshot: mount.snapshot.clone(),
    }
}
fn started(
    expiry: bool,
) -> (
    Grant,
    crate::device_authorization_lease::Lease,
    MountedSurface,
    Runtime,
) {
    let (listener, client, config) = fixture();
    let source = crate::surface_javascript::fixture::SOURCE.as_bytes();
    let mut mount = script(&client.origin, source);
    if expiry {
        mutate(&mut mount, |s| {
            s["resourceLeases"][0]["expiresAtMs"] = (clock() + 3000).into()
        });
    }
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let destination = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, source);
    });
    let mut runtime = Runtime::start_prepared(
        client.clone(),
        grant.lease(&client.origin, clock()).unwrap(),
        &destination,
        copy(&mount),
    )
    .unwrap();
    wait_reply(&mut runtime);
    peer.join().unwrap();
    (grant, destination, mount, runtime)
}
fn wait_reply(runtime: &mut Runtime) {
    runtime.wait_reply_for_test().unwrap();
}
fn finish(runtime: &mut Runtime) {
    let handle = runtime.wait_handles().next().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 8000) },
        WAIT_OBJECT_0
    );
    assert!(runtime.take_exit().is_some());
    assert!(runtime.finished() && runtime.wait_handles().next().is_none());
}
#[test]
fn javascript_runtime_mount_rebind_keeps_child_and_blocks_unconsumed_operations() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, destination, mut mount, mut runtime) = started(false);
    let probe = runtime.probe().unwrap();
    assert!(runtime.pending());
    assert_eq!(
        runtime.rebind(copy(&mount)).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let frame = runtime
        .take_reply(&destination, &mount)
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:42:0"
    );
    assert!(!runtime.pending());
    assert!(runtime.take_reply(&destination, &mount).is_none());
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().nth(1).unwrap().as_raw_handle()),
                0,
            )
        },
        windows::Win32::Foundation::WAIT_TIMEOUT
    );
    activate(&mut mount);
    runtime.rebind(copy(&mount)).unwrap();
    wait_reply(&mut runtime);
    assert!(runtime
        .take_reply(&destination, &mount)
        .unwrap()
        .unwrap()
        .is_none());
    assert!(!probe.exited());
    runtime.request_stop();
    finish(&mut runtime);
    assert!(probe.exited());
}
#[test]
fn javascript_runtime_reply_wait_preserves_operation_failure_and_reaps_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, destination, mut mount, mut runtime) = started(false);
    runtime.take_reply(&destination, &mount).unwrap().unwrap();
    let probe = runtime.probe().unwrap();
    activate(&mut mount);
    runtime.rebind(copy(&mount)).unwrap();
    wait_reply(&mut runtime);
    runtime.take_reply(&destination, &mount).unwrap().unwrap();
    runtime
        .click(
            copy(&mount),
            "missing".into(),
            crate::surface_viewport::Viewport::new([800, 600], 96).unwrap(),
        )
        .unwrap();
    assert_eq!(
        runtime.wait_reply_for_test(),
        Err(crate::surface_javascript::authorized::Error::Execution(
            std::io::ErrorKind::InvalidData
        ))
    );
    assert!(runtime.finished() && !runtime.pending() && probe.exited());
    assert_eq!(runtime.wait_handles().count(), 0);
}
#[test]
fn javascript_runtime_idle_revocation_and_source_expiry_reap_without_ui_refresh() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for expiry in [false, true] {
        let (grant, destination, mount, mut runtime) = started(expiry);
        runtime.take_reply(&destination, &mount).unwrap().unwrap();
        let probe = runtime.probe().unwrap();
        if !expiry {
            drop(grant);
        }
        finish(&mut runtime);
        assert!(probe.exited());
    }
}
#[test]
fn javascript_runtime_pending_frame_rejects_foreign_grant_and_lifecycle_change() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for scenario in 0..4 {
        let (_grant, destination, mut mount, mut runtime) = started(false);
        let probe = runtime.probe().unwrap();
        let mut other = Grant::new(session(mount.binding.origin(), u64::MAX));
        let replacement = other.lease(mount.binding.origin(), clock()).unwrap();
        match scenario {
            0 => activate(&mut mount),
            2 => mutate(&mut mount, |s| {
                s["resourceLeases"][0]["leaseId"] = "changed".into()
            }),
            3 => mutate(&mut mount, |s| s["state"]["quote"] = 900.into()),
            _ => {}
        }
        assert!(runtime
            .take_reply(
                if scenario == 1 {
                    &replacement
                } else {
                    &destination
                },
                &mount
            )
            .unwrap()
            .is_err());
        finish(&mut runtime);
        assert!(probe.exited());
    }
}
#[test]
fn javascript_runtime_refuses_foreign_destination_before_network_or_thread() {
    let (listener, client, _) = fixture();
    let mount = script(&client.origin, b"unused");
    let mut source = Grant::new(session(&client.origin, u64::MAX));
    let mut other = Grant::new(session(&client.origin, u64::MAX));
    assert!(Runtime::start_prepared(
        client.clone(),
        source.lease(&client.origin, clock()).unwrap(),
        &other.lease(&client.origin, clock()).unwrap(),
        mount
    )
    .is_err());
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}
#[test]
fn javascript_runtime_cancel_download_reaps_background_owner_without_starting_child() {
    let (listener, client, config) = fixture();
    let mount = script(&client.origin, b"never delivered");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let destination = grant.lease(&client.origin, clock()).unwrap();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        entered_tx.send(()).unwrap();
        finish_rx
            .recv_timeout(std::time::Duration::from_secs(8))
            .unwrap();
    });
    let mut runtime = Runtime::start_prepared(
        client.clone(),
        grant.lease(&client.origin, clock()).unwrap(),
        &destination,
        mount,
    )
    .unwrap();
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    runtime.request_stop();
    finish(&mut runtime);
    assert!(runtime.probe().is_none());
    finish_tx.send(()).unwrap();
    peer.join().unwrap();
}
