//! 真实 HWND 生命周期与消息测试；不打开系统历史，不替代人工输入验收。
use super::*;
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("menu test"),
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn rows() -> Vec<Row> {
    vec![Row {
        key: 0,
        thumbnail: None,
        label: "原生菜单测试(&T)\tCtrl+Shift+C".into(),
        enabled: true,
        checked: true,
        submenu: false,
        separator: false,
        focused: false,
        radio: true,
        default: true,
    }]
}
fn panel(owner: HWND) -> Panel {
    Panel::create(
        owner,
        RECT {
            left: 20,
            top: 20,
            right: 260,
            bottom: 80,
        },
        30,
        rows(),
    )
    .unwrap()
}

#[test]
fn notification_reentrancy_cannot_keep_a_destroyed_selection_valid() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    assert!(panel.notification_guard(|| {}));
    assert!(!panel.notification_guard(|| unsafe {
        DestroyWindow(panel.hwnd).unwrap();
    }));
    assert!(!panel.notification_guard(|| panic!("destroyed panel must not notify")));
}

#[test]
fn notification_reentrancy_preserves_cancel_even_if_hwnd_and_capture_stay_unchanged() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    assert!(!panel.notification_guard(|| unsafe {
        SendMessageW(panel.hwnd, WM_CANCELMODE, None, None);
    }));
    assert!(panel.alive());
    assert!(!panel.notification_guard(|| panic!("pending cancellation must not notify")));
}

#[test]
fn real_panel_paints_routes_messages_and_destroys_before_data() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    let hwnd = panel.hwnd;
    panel.show().unwrap();
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = UpdateWindow(hwnd);
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1b)), None);
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM((12 << 16) | 10)));
    }
    assert!(panel.data.paints.get() > 0);
    assert_eq!(panel.next_event(), Some(Event::Key(0x1b)));
    assert_eq!(panel.next_event(), Some(Event::Down(10, 12)));
    panel.update(rows()).unwrap();
    drop(panel);
    assert!(!unsafe { IsWindow(Some(hwnd)).as_bool() });
}
#[test]
fn real_panel_paints_shared_thumbnail_on_normal_and_focused_background() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    let image = crate::image_decode::Image {
        width: 2,
        height: 1,
        pixels: vec![10, 20, 30, 128, 0, 0, 0, 0].into(),
    };
    let thumbnail = crate::image_thumbnail::Thumbnail::from_image(&image).unwrap();
    panel.show().unwrap();
    for focused in [false, true] {
        let mut rows = rows();
        rows[0].thumbnail = Some(thumbnail.clone());
        rows[0].focused = focused;
        panel.update(rows).unwrap();
        unsafe {
            let _ = UpdateWindow(panel.hwnd);
        }
        assert!(!panel.data.failed.get());
    }
    assert!(panel.data.paints.get() >= 2);
}
#[test]
fn owner_destruction_and_input_overflow_fail_closed() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    for _ in 0..65 {
        unsafe {
            SendMessageW(panel.hwnd, WM_KEYDOWN, Some(WPARAM(40)), None);
        }
    }
    assert_eq!(panel.next_event(), Some(Event::Cancel));
    drop(owner);
    assert!(!panel.alive());
    assert!(panel.update(rows()).is_err());
    drop(panel);
}

#[test]
fn destruction_supersedes_pending_input_and_motion_is_coalesced() {
    let owner = Owner::new();
    let panel = panel(owner.0);
    for y in 0..100 {
        unsafe {
            SendMessageW(panel.hwnd, WM_MOUSEMOVE, None, Some(LPARAM(y << 16)));
        }
    }
    assert_eq!(panel.next_event(), Some(Event::Move(0, 99)));
    assert!(panel.next_event().is_none());
    unsafe {
        SendMessageW(panel.hwnd, WM_KEYDOWN, Some(WPARAM(13)), None);
    }
    drop(owner);
    assert_eq!(panel.next_event(), Some(Event::Cancel));
}

#[test]
fn real_gdi_measurement_expands_long_labels_and_shortcut_columns() {
    let owner = Owner::new();
    let short = Panel::preferred_width(owner.0, &rows()).unwrap();
    let mut long = rows();
    long[0].label = format!("{}\tCtrl+Shift+Alt+F12", "长菜单标签".repeat(20));
    assert!(Panel::preferred_width(owner.0, &long).unwrap() > short);
    long[0].label = "x".repeat(128 * 1024 + 1);
    assert!(Panel::preferred_width(owner.0, &long).is_err());
}
