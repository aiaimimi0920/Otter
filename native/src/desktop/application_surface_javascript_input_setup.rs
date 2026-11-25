//! Real mount/hidden DOM/Active ACK preparation for Application JS tests, without enabling input or replacing production admission.
use super::*;
pub(super) fn wait_batch(app: &Application) {
    // One batch contains two submissions and two observations; use the original product's total 10s+25s budget, not a single-request fixture's 5s.
    let handles: Vec<_> = app.remote_sessions.wait_handles().collect();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(
                HANDLE(handles[0].as_raw_handle()),
                36000,
            )
        },
        WAIT_OBJECT_0
    );
}
pub(super) unsafe fn mount(app: &mut Application, origin: &str, root: &std::path::Path) -> HWND {
    let now = crate::desktop::surface_windows::now_ms().unwrap();
    let request = crate::surface_attach_runtime::Request::new(
        "art:test".into(),
        "node:test".into(),
        peer::report(),
        crate::live_relay_http::Persistence::Temporary,
    )
    .unwrap();
    app.remote_sessions
        .start_attach(
            fixture::config(origin, root).unwrap(),
            app.authorization_owner.lease(origin, now).unwrap(),
            std::sync::Arc::new(request),
        )
        .unwrap();
    fixture::wait(app);
    app.poll_remote_live_with(false, |o| fixture::config(o, root))
        .unwrap();
    let parent = app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .edit_test_parent();
    SetWindowPos(
        parent,
        None,
        -32000,
        -32000,
        0,
        0,
        SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    // attach creates windows only after the previous JS scan; run the original pump again to start Runtime without bypassing capacity or leases.
    app.poll_remote_live_with(false, |o| fixture::config(o, root))
        .unwrap();
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(
        window.javascript_pending_for_test(),
        "JS fixture did not start: needs_start={}, activation_error={:?}",
        window.javascript_needs_start(),
        window.activation_error()
    );
    wait_js(app, true);
    app.poll_remote_live_with(false, |o| fixture::config(o, root))
        .unwrap();
    fixture::wait(app);
    app.poll_remote_live_with(false, |o| fixture::config(o, root))
        .unwrap();
    if app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test()
    {
        wait_js(app, true);
    }
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    parent
}
