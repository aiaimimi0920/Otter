//! 应用集合的资源任务准入、等待、取消和退出；使用已撤销租约，禁止任何网络 I/O。
use super::*;
fn resource(index: usize) -> crate::live_relay_http::ResourceLease {
    let digest = format!("{index:064x}");
    let id = format!("sha256:{digest}");
    let mount = crate::live_relay_http::MountedSurface {
        binding: surface_owner().binding().clone(),
        snapshot: serde_json::value::to_raw_value(&serde_json::json!({
            "protocolVersion":"loom.surface.v1", "instanceId":"surface:test",
            "attachmentId":"attachment:test", "revision":1,
            "resourceLeases":[{"leaseId":"test", "expiresAtMs":1000,
                "resource":{"resourceId":id,"kind":"image","mime":"image/png","size":4},
                "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{digest}")}}]
        })).unwrap(),
    };
    mount.image_resource(&id, 0).unwrap()
}
#[test]
fn resource_jobs_share_session_capacity_wait_and_close_drain() {
    let mut sessions = Sessions::default();
    for i in 0..MAX_SESSIONS {
        sessions
            .start_surface_resource(surface_config(), revoked(), resource(i))
            .unwrap();
    }
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert_eq!(sessions.surface_shutdown_handles().count(), MAX_SESSIONS);
    assert!(sessions.surface_cleanup_pending());
    assert!(
        matches!(sessions.start_surface_resource(surface_config(), revoked(), resource(8)),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    sessions.close();
    assert!(sessions.resource_deadline(0).is_none());
    while !sessions.finished() {
        assert!(sessions.poll_surfaces().is_empty());
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert_eq!(sessions.occupied(), 0);
    assert_eq!(sessions.wait_handles().count(), 0);
    assert_eq!(sessions.surface_shutdown_handles().count(), 0);
    assert!(!sessions.surface_cleanup_pending());
    assert!(!sessions.deliver_surface_resource(|_| panic!("cancelled GET delivered")));
    assert!(
        matches!(sessions.start_surface_resource(surface_config(), revoked(), resource(0)),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe)
    );
}
#[test]
fn duplicate_resource_job_is_rejected_without_allocating_another_slot() {
    let mut sessions = Sessions::default();
    sessions
        .start_surface_resource(surface_config(), revoked(), resource(0))
        .unwrap();
    assert!(
        matches!(sessions.start_surface_resource(surface_config(), revoked(), resource(0)),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists)
    );
    assert_eq!(sessions.occupied(), 1);
    sessions.cancel_surface_resources(|binding, revision| {
        binding.node() == "node:test" && revision == 1
    });
    let handles: Vec<_> = sessions
        .wait_handles()
        .map(|h| HANDLE(h.as_raw_handle()))
        .collect();
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handles, false, 5000) },
        WAIT_OBJECT_0
    );
    assert!(sessions.poll_surfaces().is_empty());
    assert!(sessions.finished());
}
