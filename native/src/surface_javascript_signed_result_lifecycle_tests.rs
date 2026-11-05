//! 已排队的正式结果仍须领取时授权；换工作区/撤权不发布，原 child/像素随终止回收。
use super::*;
#[test]
fn queued_result_is_rejected_after_revocation_or_workspace_change() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for revoke in [false, true] {
        let source = "NeuroSurface.define({mount(){NeuroSurface.commitResult({answer:{kind:'value',value:42}})}});";
        let (owner, lease) = fixture::lease();
        let mut grant = fixture::grant(source);
        grant
            .permissions
            .push(crate::art_execution_grant::wire::Permission::WorkspaceResult);
        let current = grant.binding.clone();
        let worker_lease = lease.clone();
        let mut runtime = Runtime::spawn(
            current.clone(),
            lease,
            (
                grant.expires_at_ms,
                Instant::now() + Duration::from_secs(60),
            ),
            true,
            move || fixture::mount(&grant, source, worker_lease, &|| false).map_err(Error::Local),
        )
        .unwrap();
        let handle = [HANDLE(runtime.mailbox.reply_handle().as_raw_handle())];
        assert_eq!(
            unsafe { WaitForMultipleObjects(&handle, false, 8000) },
            WAIT_OBJECT_0
        );
        let mut current = current;
        if revoke {
            drop(owner);
        } else {
            current.workspace_id = "workspace:replacement".into();
        }
        assert!(matches!(
            runtime.take_reply(&current),
            Some(Err(Error::Stopped))
        ));
        exit(&mut runtime);
        assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
    }
}
