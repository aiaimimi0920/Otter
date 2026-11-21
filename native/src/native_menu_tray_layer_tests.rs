//! 自有置顶遮挡窗证明托盘菜单层级；不操作 Explorer 或用户前台。
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
                // STATIC 默认命中穿透；对照窗必须像真实托盘浮层一样接收命中。
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
    // 原普通层级确实被对照浮层遮挡，避免只断言样式而未验证命中。
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
                // 私有桌面不抢前台；只替换激活准入，后续捕获/消息泵均为产品路径。
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
