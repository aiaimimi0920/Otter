//! timer 复用原应用 wait/deadline、策略、唯一 owner 和 TLS 两事件 FIFO 夹具。
use super::*;
pub(super) unsafe fn prepare(app: &mut Application, root: &std::path::Path) {
    std::thread::sleep(std::time::Duration::from_millis(25));
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(window.javascript_timer_queued_for_test());
    assert!(!window.javascript_animation_queued_for_test());
    animation::paused(app);
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    animation::tick(app, root);
    animation::tick(app, root);
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(!window.javascript_timer_queued_for_test());
    assert!(!window.javascript_animation_queued_for_test());
}
#[test]
fn application_javascript_timer_deadline_fifo_same_child_modal_and_owner_fences() {
    run(11);
}
#[test]
fn application_javascript_timer_stale_viewport_rejects_without_replay_or_emit() {
    run(12);
}
