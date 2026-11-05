//! 原 TLS/Application owner 的 observer→rAF→Canvas 像素与空闲回收；不制造服务端业务结果。
use super::*;
pub(super) unsafe fn exercise(app: &mut Application, root: &std::path::Path) {
    {
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert_eq!(
            window.javascript_canvas_pixel_for_test(2, 2),
            [0, 255, 0, 255]
        );
        assert_eq!(window.javascript_resize_report_for_test()["frames"], 0);
    }
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    animation::tick(app, root);
    {
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert_eq!(
            window.javascript_canvas_pixel_for_test(2, 2),
            [255, 0, 0, 255]
        );
        assert_eq!(window.javascript_resize_report_for_test()["frames"], 1);
        assert!(!window.javascript_pending_for_test());
        assert!(!window.javascript_animation_queued_for_test());
    }
    for _ in 0..3 {
        app.poll_remote_live_with(true, |_| panic!("idle canvas caused network work"))
            .unwrap();
    }
    assert_eq!(app.remote_sessions.surface_event_backlog_for_test(), 0);
}
#[test]
fn application_javascript_canvas_real_native_pixels_observer_raf_idle_and_cleanup() {
    run(10);
}
