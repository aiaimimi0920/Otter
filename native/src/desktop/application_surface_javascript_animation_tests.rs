//! 真实应用 deadline 唤醒/模态背压/外借 owner fence；复用原 TLS peer/批次和挂载夹具。
use super::*;
// 调用者须位于夹具的 COM/UI owner 线程，沿用原 Application polling 的 HWND 安全前提。
pub(super) unsafe fn tick(app: &mut Application, root: &std::path::Path) {
    if !app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test()
    {
        let deadline = app
            .surface_windows
            .deadline(crate::desktop::surface_windows::now_ms().unwrap())
            .unwrap()
            .unwrap();
        let delay = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(delay < std::time::Duration::from_secs(1));
        std::thread::sleep(delay + std::time::Duration::from_millis(1));
        app.poll_remote_live_with(true, |o| fixture::config(o, root))
            .unwrap();
        assert!(app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_pending_for_test());
    }
    wait_js(app, true);
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
}
pub(super) fn paused(app: &mut Application) {
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(
        window.javascript_animation_queued_for_test() || window.javascript_timer_queued_for_test()
    );
    assert!(!window.javascript_pending_for_test());
    let deadline = app
        .surface_windows
        .deadline(crate::desktop::surface_windows::now_ms().unwrap())
        .unwrap()
        .unwrap();
    assert!(
        deadline.saturating_duration_since(std::time::Instant::now())
            > std::time::Duration::from_secs(1)
    );
}
pub(super) unsafe fn prepare(app: &mut Application, root: &std::path::Path) {
    app.authorization.borrow_mut().busy = true;
    std::thread::sleep(std::time::Duration::from_millis(25));
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    paused(app);
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    tick(app, root);
    tick(app, root);
    assert!(!app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_animation_queued_for_test());
}
pub(super) unsafe fn stale(app: &mut Application, root: &std::path::Path, parent: HWND) {
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    if !app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test()
    {
        let due = app
            .surface_windows
            .deadline(crate::desktop::surface_windows::now_ms().unwrap())
            .unwrap()
            .unwrap();
        let delay = due.saturating_duration_since(std::time::Instant::now());
        assert!(delay < std::time::Duration::from_secs(1));
        std::thread::sleep(delay + std::time::Duration::from_millis(1));
        app.poll_remote_live_with(true, |o| fixture::config(o, root))
            .unwrap();
    }
    assert!(app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test());
    wait_js(app, true);
    SetWindowPos(
        parent,
        None,
        0,
        0,
        470,
        280,
        SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(!window.is_alive() && window.snapshot_revision() == 1);
    // JS 可能已回收并在同轮排入 inactive；聚合等待句柄不等于 JS 事件任务。
    // 直接保护零事件/收据，统一 teardown 的 peer 还须确认唯一 inactive 且无 events。
    assert_eq!(app.remote_sessions.surface_event_backlog_for_test(), 0);
}
#[test]
fn application_javascript_animation_deadline_frames_emit_fifo_modal_and_owner_fences() {
    run(6);
}
#[test]
fn application_javascript_animation_stale_viewport_closes_without_replaying_frame() {
    run(7);
}
