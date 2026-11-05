//! 真实 common-control 鼠标消息与同步通知；不向用户桌面注入输入。
use super::*;
fn spinner(window: &SurfaceWindow) -> HWND {
    unsafe { FindWindowExW(Some(window.hwnd), None, UPDOWN_CLASSW, None).unwrap() }
}
fn click(hwnd: HWND, up: bool) {
    unsafe {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).unwrap();
        let point = LPARAM(
            ((if up {
                rect.bottom / 4
            } else {
                rect.bottom * 3 / 4
            }) << 16
                | (rect.right / 2)) as isize,
        );
        SendMessageW(hwnd, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point));
    }
}
fn notify(window: &SurfaceWindow, hwnd: HWND, delta: i32) -> LRESULT {
    let notification = NMUPDOWN {
        hdr: NMHDR {
            hwndFrom: hwnd,
            idFrom: 0,
            code: UDN_DELTAPOS,
        },
        iPos: 0,
        iDelta: delta,
    };
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_NOTIFY,
            None,
            Some(LPARAM((&notification as *const NMUPDOWN) as isize)),
        )
    }
}
#[test]
fn surface_number_spinner_mouse_steps_float_without_native_integer_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1.5"));
    let edit = child(&window);
    let spin = spinner(&window);
    click(spin, true);
    assert_eq!(text(edit), "2");
    assert_eq!(unsafe { GetFocus() }, edit);
    let (events, input) = event(&window);
    assert_eq!(input["event"], "input");
    assert_eq!(input["payload"]["value"], 2.);
    window.settle_edits(&events).unwrap();
    let (events, change) = event(&window);
    assert_eq!(change["event"], "change");
    window.replace(form(serde_json::json!("2"), 0.5)).unwrap();
    window.settle_edits(&events).unwrap();
    click(spin, false);
    assert_eq!(text(edit), "1.5");
    assert_eq!(unsafe { SendMessageW(spin, UDM_GETPOS32, None, None) }.0, 0);
    assert!(window.take_error().is_none());
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(spin)) }.as_bool());
}
#[test]
fn surface_number_spinner_rechecks_policy_composition_identity_and_busy_host() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1.5"));
    let edit = child(&window);
    let spin = spinner(&window);
    assert_eq!(notify(&window, edit, 1).0, 1);
    assert_eq!(notify(&window, spin, 2).0, 1);
    window
        .state
        .run(|_| {
            assert_eq!(notify(&window, spin, 1).0, 1);
            Ok(())
        })
        .unwrap();
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    notify(&window, spin, 1);
    assert_eq!(text(edit), "1.5");
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    window.set_input_policy(false, false).unwrap();
    assert!(!unsafe { IsWindowEnabled(spin) }.as_bool());
    notify(&window, spin, 1);
    assert_eq!(text(edit), "1.5");
    window.set_input_policy(true, false).unwrap();
    click(spin, true);
    assert_eq!(text(edit), "2");
    assert!(window.prepare_edit(&binding(), 1).is_err());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_spinner_partitions_geometry_and_cancels_capture() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1.5"));
    let edit = child(&window);
    let spin = spinner(&window);
    unsafe {
        let mut a = RECT::default();
        let mut b = RECT::default();
        GetWindowRect(edit, &mut a).unwrap();
        GetWindowRect(spin, &mut b).unwrap();
        assert_eq!(a.right, b.left);
        assert_eq!(a.top, b.top);
        assert_eq!(a.bottom, b.bottom);
        SetCapture(spin);
        SendMessageW(window.hwnd, WM_CANCELMODE, None, None);
        assert_ne!(GetCapture(), spin);
        SetCapture(spin);
        window.set_input_policy(false, false).unwrap();
        assert_ne!(GetCapture(), spin);
    }
    window.replace(super::super::form("plain", false)).unwrap();
    assert!(!unsafe { IsWindow(Some(spin)) }.as_bool());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_spinner_fully_occluded_region_rejects_late_notification() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"stack","layout":{"width":180,"height":80},"children":[{"id":"field","type":"number","props":{"value":"1","min":0,"max":10},"layout":{"width":180,"height":32},"events":{"input":"typing"}},{"id":"cover","type":"view","layout":{"width":180,"height":40}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let spin = spinner(&window);
    assert!(!unsafe { IsWindowEnabled(spin) }.as_bool());
    assert_eq!(notify(&window, spin, 1).0, 1);
    assert_eq!(text(child(&window)), "1");
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
