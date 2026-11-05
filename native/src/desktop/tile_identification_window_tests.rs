//! 自有离屏HWND真实paint/关闭输入；不抢焦点，不向用户程序发键鼠，不截图桌面。
use super::*;
fn fixture() -> (
    Arc<Shared>,
    crate::tile_outputs::TileOutput,
    crate::tile_wire::Endpoint,
) {
    let shared = Arc::new(Shared::default());
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 640, 480);
    let endpoint = crate::tile_wire::Endpoint::image("device-1", &output);
    let t = Instant::now();
    let mut s = shared.lock();
    s.until = Some(t + std::time::Duration::from_secs(6));
    s.identification.observe(
        Some(&crate::tile_wire::Identification {
            request_id: "11111111-1111-4111-8111-111111111111".into(),
            remaining_ms: 1000,
            applied: false,
        }),
        &endpoint,
        "lease",
        t,
        t,
    );
    drop(s);
    (shared, output, endpoint)
}
#[test]
fn unassigned_output_reports_only_after_real_paint_and_escape_dismisses_without_exit() {
    let _com = Apartment::new();
    let (shared, output, endpoint) = fixture();
    assert!(shared
        .lock()
        .identification
        .take_report(&endpoint.endpoint_id, "lease", Instant::now())
        .is_none());
    let window = Window::new(shared.clone(), &output).unwrap();
    unsafe {
        assert!(!IsWindowVisible(window.hwnd).as_bool());
        SendMessageW(window.hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
    }
    assert!(!window.state.painted.get());
    assert!(shared
        .lock()
        .identification
        .take_report(&endpoint.endpoint_id, "lease", Instant::now())
        .is_none());
    window.tick().unwrap();
    window.tick().unwrap();
    assert!(window.state.painted.get());
    assert_ne!(unsafe { GetForegroundWindow() }, window.hwnd);
    let report = shared
        .lock()
        .identification
        .take_report(&endpoint.endpoint_id, "lease", Instant::now())
        .unwrap();
    assert_eq!(report.outcome, crate::tile_identification::Outcome::Applied);
    {
        let s = shared.lock();
        assert!(s.frame.is_none() && s.presented.is_none());
    }
    unsafe {
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(27)), Some(LPARAM(0)));
        // 同一按键的repeat/迟到消息在新鲜普通周期之前不能把输出顺便关闭。
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(27)), Some(LPARAM(0)));
    }
    assert!(window.alive());
    window.tick().unwrap();
    assert!(window.state.marker.borrow().is_none());
    assert_eq!(
        shared
            .lock()
            .identification
            .take_report(&endpoint.endpoint_id, "lease", Instant::now())
            .unwrap()
            .outcome,
        crate::tile_identification::Outcome::Dismissed
    );
    {
        let mut s = shared.lock();
        s.identification
            .observe(None, &endpoint, "lease", Instant::now(), Instant::now());
        s.identification.finish_cycle();
    }
    unsafe {
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(27)), Some(LPARAM(0)));
    }
    assert!(!window.alive() && shared.lock().stopped);
}
#[test]
fn close_button_uses_current_half_open_bounds_and_authorization_loss_clears_it() {
    let _com = Apartment::new();
    let (shared, output, endpoint) = fixture();
    let window = Window::new(shared.clone(), &output).unwrap();
    window.tick().unwrap();
    window.tick().unwrap();
    unsafe {
        // 右边界x=576不在按钮内；内点x=320,y=420关闭识别，不关输出。
        SendMessageW(
            window.hwnd,
            WM_LBUTTONUP,
            Some(WPARAM(0)),
            Some(LPARAM(576 | (420 << 16))),
        );
        assert!(shared
            .lock()
            .identification
            .marker(Instant::now())
            .is_some());
        SendMessageW(
            window.hwnd,
            WM_LBUTTONUP,
            Some(WPARAM(0)),
            Some(LPARAM(320 | (420 << 16))),
        );
    }
    assert!(shared
        .lock()
        .identification
        .marker(Instant::now())
        .is_none());
    assert!(window.alive());
    assert_eq!(
        shared
            .lock()
            .identification
            .take_report(&endpoint.endpoint_id, "lease", Instant::now())
            .unwrap()
            .outcome,
        crate::tile_identification::Outcome::Dismissed
    );
    shared.stop();
    window.tick().unwrap();
    assert!(window.state.marker.borrow().is_none());
    assert!(shared
        .lock()
        .identification
        .take_report(&endpoint.endpoint_id, "lease", Instant::now())
        .is_none());
}
#[test]
fn marker_paint_display_reentry_cannot_acknowledge_or_restore_old_output() {
    let _com = Apartment::new();
    let (shared, output, endpoint) = fixture();
    let window = Window::new(shared.clone(), &output).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        SetWindowLongPtrW(
            window.hwnd,
            GWLP_WNDPROC,
            change_during_paint as *const () as isize,
        );
    }
    window.tick().unwrap();
    assert!(window.output_changed() && shared.lock().stopped);
    assert!(window.state.renderer.borrow().is_none());
    assert!(shared
        .lock()
        .identification
        .take_report(&endpoint.endpoint_id, "lease", Instant::now())
        .is_none());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
}
#[test]
fn old_displayed_close_and_escape_cannot_dismiss_a_not_yet_painted_successor() {
    let _com = Apartment::new();
    let (shared, output, endpoint) = fixture();
    let window = Window::new(shared.clone(), &output).unwrap();
    window.tick().unwrap();
    window.tick().unwrap();
    let now = Instant::now();
    shared.lock().identification.observe(
        Some(&crate::tile_wire::Identification {
            request_id: "22222222-2222-4222-8222-222222222222".into(),
            remaining_ms: 1000,
            applied: false,
        }),
        &endpoint,
        "lease",
        now,
        now,
    );
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_LBUTTONUP,
            Some(WPARAM(0)),
            Some(LPARAM(320 | (420 << 16))),
        );
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(27)), Some(LPARAM(0)));
    }
    let mut s = shared.lock();
    assert_eq!(
        s.identification.marker(now).unwrap().identity.request,
        "22222222-2222-4222-8222-222222222222"
    );
    assert!(s
        .identification
        .take_report(&endpoint.endpoint_id, "lease", now)
        .is_none());
}
