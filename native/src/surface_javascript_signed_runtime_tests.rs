//! 真实 child 的后台挂载、单槽背压、闲置撤权/期限与终态回收；不以它替代 HWND 集成。
use super::super::tests as fixture;
use super::*;
use std::{os::windows::io::AsRawHandle, sync::mpsc, time::Duration};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForMultipleObjects,
};
#[path = "surface_javascript_signed_interaction_tests.rs"]
mod interaction;
#[path = "surface_javascript_signed_runtime_lifecycle_tests.rs"]
mod lifecycle;
#[path = "surface_javascript_signed_live_trigger_tests.rs"]
mod live_trigger;
#[path = "surface_javascript_signed_result_lifecycle_tests.rs"]
mod result_lifecycle;
pub(crate) fn start(
    deadline: Option<Duration>,
) -> (
    Runtime,
    crate::device_authorization_lease::Grant,
    mpsc::Receiver<crate::bounded_worker_process::Probe>,
) {
    start_notified(deadline, Arc::new(|| {}))
}
fn start_notified(
    deadline: Option<Duration>,
    notify: Arc<dyn Fn() + Send + Sync>,
) -> (
    Runtime,
    crate::device_authorization_lease::Grant,
    mpsc::Receiver<crate::bounded_worker_process::Probe>,
) {
    start_fixture(deadline, notify, fixture::SOURCE)
}
pub(crate) fn start_source(
    source: &'static str,
) -> (
    Runtime,
    crate::device_authorization_lease::Grant,
    mpsc::Receiver<crate::bounded_worker_process::Probe>,
) {
    start_fixture(None, Arc::new(|| {}), source)
}
fn start_fixture(
    deadline: Option<Duration>,
    notify: Arc<dyn Fn() + Send + Sync>,
    source: &'static str,
) -> (
    Runtime,
    crate::device_authorization_lease::Grant,
    mpsc::Receiver<crate::bounded_worker_process::Probe>,
) {
    let (owner, lease) = fixture::lease();
    let grant = fixture::grant(source);
    let binding = grant.binding.clone();
    let worker_lease = lease.clone();
    let (tx, rx) = mpsc::channel();
    let limits = (
        grant.expires_at_ms,
        Instant::now() + deadline.unwrap_or(Duration::from_secs(60)),
    );
    let runtime = Runtime::spawn_notified(binding, lease, limits, true, notify, move || {
        let (local, frame) =
            fixture::mount(&grant, source, worker_lease, &|| false).map_err(Error::Local)?;
        let probe = local
            .session
            .as_ref()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .probe()
            .unwrap();
        tx.send(probe).unwrap();
        Ok((local, frame))
    })
    .unwrap();
    (runtime, owner, rx)
}
fn reply(runtime: &mut Runtime) -> Frame {
    let handles: Vec<_> = runtime
        .wait_handles()
        .map(|h| HANDLE(h.as_raw_handle()))
        .collect();
    assert_eq!(handles.len(), 2);
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handles, false, 8000) }.0,
        WAIT_OBJECT_0.0 + 1
    );
    runtime
        .take_reply(&runtime.binding.clone())
        .unwrap()
        .unwrap()
}
fn exit(runtime: &mut Runtime) {
    let thread = runtime.thread.as_ref().unwrap();
    let handle = [HANDLE(thread.as_handle().as_raw_handle())];
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handle, false, 8000) },
        WAIT_OBJECT_0
    );
    assert!(runtime.take_exit().unwrap().is_err());
    assert!(runtime.finished());
}
#[test]
fn background_mount_click_backpressure_and_idle_revocation_reclaim_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, owner, probe) = start(None);
    let current = runtime.binding.clone();
    assert_eq!(
        runtime.submit(&current, Operation::Suspend),
        Err(Error::Io(io::ErrorKind::WouldBlock))
    );
    let first = reply(&mut runtime);
    assert_eq!(crate::surface_javascript::fixture::text(&first.scene), "0");
    let probe = probe.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime
        .submit(
            &current,
            Operation::Click(first.scene.root().children()[1].id().into()),
        )
        .unwrap();
    assert_eq!(
        runtime.submit(&current, Operation::Resume),
        Err(Error::Io(io::ErrorKind::WouldBlock))
    );
    assert_eq!(
        crate::surface_javascript::fixture::text(&reply(&mut runtime).scene),
        "1"
    );
    drop(owner);
    exit(&mut runtime);
    assert!(probe.exited());
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}
#[test]
fn changed_host_binding_rejects_ready_frame_and_stops_original_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _owner, probe) = start(None);
    let handle = [HANDLE(runtime.mailbox.reply_handle().as_raw_handle())];
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handle, false, 8000) },
        WAIT_OBJECT_0
    );
    let mut other = runtime.binding.clone();
    other.workspace_id = "workspace:other".into();
    assert!(matches!(
        runtime.take_reply(&other),
        Some(Err(Error::Stopped))
    ));
    exit(&mut runtime);
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
#[test]
fn idle_monotonic_deadline_wakes_without_an_operation_or_polling() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _owner, probe) = start(Some(Duration::from_secs(2)));
    // 不要求调度在期限前交首帧；无论挂载快慢，都必须在原固定期限后退出并回收。
    exit(&mut runtime);
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
#[test]
fn mount_panic_and_explicit_stop_never_leave_a_live_thread() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = fixture::lease();
    let grant = fixture::grant(fixture::SOURCE);
    let mut runtime = Runtime::spawn(
        grant.binding,
        lease.clone(),
        (
            grant.expires_at_ms,
            Instant::now() + Duration::from_secs(60),
        ),
        true,
        || panic!("injected mount panic"),
    )
    .unwrap();
    exit(&mut runtime);
    assert!(lease.is_revoked());
    let (mut runtime, _owner, probe) = start(None);
    let _frame = reply(&mut runtime);
    runtime.request_stop();
    exit(&mut runtime);
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
