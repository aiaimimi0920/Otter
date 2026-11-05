//! 真 TLS/Application 无点击 resize→observer→下一帧；模态保留脏代次，ABA 旧结果终态拒绝。
use super::*;
fn report(app: &mut Application) -> serde_json::Value {
    app.surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_resize_report_for_test()
}
fn idle(app: &mut Application) {
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(!window.javascript_pending_for_test());
    assert!(!window.javascript_animation_queued_for_test());
    assert!(window.wake_at_ms() > crate::desktop::surface_windows::now_ms().unwrap() + 1000);
}
pub(super) unsafe fn exercise(
    app: &mut Application,
    root: &std::path::Path,
    parent: HWND,
    stale: bool,
) {
    assert_eq!(report(app)["count"], 1);
    assert_eq!(report(app)["frames"], 0);
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    animation::tick(app, root);
    assert_eq!(report(app)["frames"], 1);
    idle(app);
    SetWindowPos(
        parent,
        None,
        0,
        0,
        600,
        320,
        SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    app.authorization.borrow_mut().busy = true;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    idle(app); // dirty observer 尚在，但模态不能贡献立即 deadline 或抢单槽。
    assert_eq!(report(app)["count"], 1);
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    assert!(app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test());
    wait_js(app, true);
    if stale {
        // resize-only operation（没有待 rAF）已经完成；A→B→A 必须仍然拒绝。
        SetWindowPos(
            parent,
            None,
            0,
            0,
            650,
            350,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        SetWindowPos(
            parent,
            None,
            0,
            0,
            600,
            320,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        app.poll_remote_live_with(true, |o| fixture::config(o, root))
            .unwrap();
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert!(!window.is_alive());
        assert_eq!(window.snapshot_revision(), 1);
    } else {
        app.poll_remote_live_with(true, |o| fixture::config(o, root))
            .unwrap();
        assert_eq!(report(app)["count"], 2);
        assert_eq!(report(app)["frames"], 1);
        animation::tick(app, root);
        let value = report(app);
        let view = crate::surface_viewport::Viewport::from_window(parent).unwrap();
        assert_eq!(value["frames"], 2);
        assert!(
            (value["bounds"].as_f64().unwrap() - f64::from(view.logical()[0] * 0.5)).abs() < 0.01
        );
        assert_eq!(
            value["bounds"].as_f64().unwrap() - 12.,
            value["entry"]["content"].as_f64().unwrap()
        );
        idle(app);
        for _ in 0..3 {
            app.poll_remote_live_with(true, |_| panic!("idle observer caused network work"))
                .unwrap();
            idle(app);
        }
        assert_eq!(report(app)["count"], 2);
    }
    assert_eq!(app.remote_sessions.surface_event_backlog_for_test(), 0);
}
#[test]
fn application_javascript_resize_no_click_real_window_observer_raf_modal_and_idle() {
    run(8);
}
#[test]
fn application_javascript_resize_observer_only_inflight_aba_closes_without_replay() {
    run(9);
}
