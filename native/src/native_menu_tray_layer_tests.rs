//! Owned topmost occlusion windows prove tray-menu z-order; no Explorer/user-foreground operations.
use super::*;

fn topmost(hwnd: HWND) -> bool {
    unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
}
fn assert_hidden_owner(owner: HWND) {
    assert!(!unsafe { IsWindowVisible(owner).as_bool() });
    assert!(!topmost(owner));
}
fn blocker() -> Owner {
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        assert!(GetMonitorInfoW(
            MonitorFromPoint(POINT { x: 30, y: 30 }, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
        .as_bool());
        let bounds = monitor.rcWork;
        let window = Owner(
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("synthetic tray overflow"),
                // STATIC defaults to hit-test transparency; control windows must receive hits like real tray overlays.
                WS_POPUP | WINDOW_STYLE(windows::Win32::System::SystemServices::SS_NOTIFY.0),
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        SetWindowPos(
            window.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .unwrap();
        window
    }
}
fn hit(page: &Page) -> HWND {
    unsafe {
        WindowFromPoint(POINT {
            x: page.bounds.left + 8,
            y: page.bounds.top + 8,
        })
    }
}

#[test]
fn tray_pages_cover_topmost_blocker_without_promoting_hidden_owner() {
    let foreground = unsafe { GetForegroundWindow() };
    let owner = Owner::new();
    let cover = blocker();
    let point = POINT { x: 30, y: 30 };
    // The control overlay must actually occlude the original normal z-order, avoiding style-only assertions without hit verification.
    let mut ordinary = Cascade::open(owner.0, tree(), point).unwrap();
    assert!(!topmost(ordinary.pages[0].panel.handle().unwrap()));
    assert_eq!(hit(&ordinary.pages[0]), cover.0);
    ordinary.cancel();
    let mut parent = tree().remove(0);
    parent.children = Some(tree());
    let mut menu = Cascade::open_with_tray_mode(owner.0, vec![parent], point, true).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    assert!(topmost(root));
    assert_eq!(hit(&menu.pages[0]), root);
    menu.event(0, Event::Move(8, 8)).unwrap();
    assert_eq!(menu.pages.len(), 2);
    let child = menu.pages[1].panel.handle().unwrap();
    assert!(topmost(child));
    assert_eq!(hit(&menu.pages[1]), child);
    assert_hidden_owner(owner.0);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    menu.cancel();
    for hwnd in [root, child] {
        assert!(!unsafe { IsWindow(Some(hwnd)).as_bool() });
    }
    assert_hidden_owner(owner.0);
}

#[test]
fn tray_layer_preparation_failure_reclaims_topmost_root() {
    let owner = Owner::new();
    let mut root = HWND::default();
    let result = run_updated(
        owner.0,
        tree(),
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        (
            |hwnd| {
                root = hwnd;
                assert!(topmost(hwnd));
                assert_hidden_owner(owner.0);
                Err(Error::new(E_ACCESSDENIED, "synthetic activation refusal"))
            },
            |_| panic!("Rejected activation must not start input"),
        ),
        |_| Ok(()),
        true,
        320,
    );
    assert!(result.is_err());
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
    assert_hidden_owner(owner.0);
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn tray_layer_queue_selects_and_reclaims_root() {
    let owner = Owner::new();
    let foreground = unsafe { GetForegroundWindow() };
    let mut root = HWND::default();
    let result = run_updated(
        owner.0,
        tree(),
        POINT { x: 30, y: 30 },
        foreground,
        (
            |hwnd| {
                root = hwnd;
                assert!(topmost(hwnd));
                assert_hidden_owner(owner.0);
                // Private desktops avoid foreground takeover; replace only activation admission, retaining product capture/message pumps.
                Ok(foreground)
            },
            |hwnd| unsafe {
                PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x28), LPARAM(0)).unwrap();
                PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x0d), LPARAM(0)).unwrap();
            },
        ),
        |_| Ok(()),
        true,
        320,
    )
    .unwrap();
    assert_eq!(result, Some(42));
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
    assert_hidden_owner(owner.0);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    assert!(unsafe { GetCapture() }.is_invalid());
}
