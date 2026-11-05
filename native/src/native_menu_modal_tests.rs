//! 真实 GetMessage/捕获/定时器生命周期；注入消息，不冒充物理输入验收。
use super::*;
use windows::core::w;
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("modal test"),
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
fn tree() -> Vec<Item> {
    vec![Item {
        command: 42,
        label: "选择".into(),
        enabled: true,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }]
}
fn start(owner: HWND, ready: impl FnOnce(HWND)) -> Result<Option<usize>> {
    run(
        owner,
        tree(),
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        ready,
    )
}

#[test]
fn queue_keyboard_returns_command_and_releases_root_capture() {
    let owner = Owner::new();
    let mut root = HWND::default();
    let result = start(owner.0, |hwnd| {
        root = hwnd;
        unsafe {
            PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x28), LPARAM(0)).unwrap();
            PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x0d), LPARAM(0)).unwrap();
        }
    })
    .unwrap();
    assert_eq!(result, Some(42));
    assert!(unsafe { GetCapture() }.is_invalid());
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
}
#[test]
fn quit_is_reposted_and_capture_loss_cancels_without_command() {
    let owner = Owner::new();
    assert_eq!(
        start(owner.0, |_| unsafe { PostQuitMessage(73) }).unwrap(),
        None
    );
    let mut message = MSG::default();
    let mut quit = None;
    // WM_QUIT 是低优先级合成消息；销毁的 WM_NULL 唤醒可以先于它到达。
    for _ in 0..64 {
        if !unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            break;
        }
        if message.message == WM_QUIT {
            quit = Some(message.wParam.0);
            break;
        }
    }
    assert_eq!(quit, Some(73));
    assert!(unsafe { GetCapture() }.is_invalid());
    assert_eq!(
        start(owner.0, |_| unsafe { ReleaseCapture().unwrap() }).unwrap(),
        None
    );
    assert!(unsafe { GetCapture() }.is_invalid());
}
#[test]
fn outside_click_owner_destruction_and_existing_capture_are_safe() {
    let owner = Owner::new();
    assert_eq!(
        start(owner.0, |hwnd| unsafe {
            PostMessageW(
                Some(hwnd),
                WM_LBUTTONDOWN,
                WPARAM(0),
                LPARAM(((-20i16 as u16 as usize) << 16 | (-20i16 as u16 as usize)) as isize),
            )
            .unwrap();
        })
        .unwrap(),
        None
    );
    unsafe {
        SetCapture(owner.0);
    }
    assert!(start(owner.0, |_| panic!("Must not steal capture")).is_err());
    assert_eq!(unsafe { GetCapture() }, owner.0);
    unsafe {
        ReleaseCapture().unwrap();
    }
    assert_eq!(
        start(owner.0, |_| unsafe { DestroyWindow(owner.0).unwrap() }).unwrap(),
        None
    );
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn captured_root_mouse_coordinates_route_to_the_hovered_child() {
    let owner = Owner::new();
    let mut parent = tree().remove(0);
    parent.children = Some(tree());
    let result = run(
        owner.0,
        vec![parent],
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        |root| unsafe {
            let mut anchor = RECT::default();
            GetWindowRect(root, &mut anchor).unwrap();
            let row = (28 * GetDpiForWindow(owner.0).clamp(48, 768) / 96) as i32;
            anchor.bottom = anchor.top + row;
            let mut child = RECT::default();
            CalculatePopupWindowPosition(
                &POINT {
                    x: anchor.right,
                    y: anchor.top,
                },
                &SIZE {
                    cx: anchor.right - anchor.left,
                    cy: row,
                },
                (TPM_VERTICAL | TPM_WORKAREA).0,
                Some(&anchor),
                &mut child,
            )
            .unwrap();
            let mut target = POINT {
                x: child.left + 4,
                y: child.top + 4,
            };
            assert!(ScreenToClient(root, &mut target).as_bool());
            let position =
                LPARAM(((target.y as u16 as usize) << 16 | target.x as u16 as usize) as isize);
            PostMessageW(Some(root), WM_MOUSEMOVE, WPARAM(0), LPARAM((4 << 16) | 4)).unwrap();
            PostMessageW(Some(root), WM_MOUSEMOVE, WPARAM(0), position).unwrap();
            PostMessageW(Some(root), WM_LBUTTONDOWN, WPARAM(0), position).unwrap();
            PostMessageW(Some(root), WM_LBUTTONUP, WPARAM(0), position).unwrap();
            // 若没有正确命中，仍通过 Escape 退出，避免错误实现造成测试无限等待。
            PostMessageW(Some(root), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
            PostMessageW(Some(root), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        },
    )
    .unwrap();
    assert_eq!(result, Some(42));
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn character_messages_execute_a_unique_mnemonic_through_the_modal_queue() {
    let owner = Owner::new();
    let mut items = tree();
    items[0].label = "执行(&Q)".into();
    let result = run(
        owner.0,
        items,
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        |root| unsafe {
            PostMessageW(Some(root), WM_CHAR, WPARAM(b'q' as usize), LPARAM(0)).unwrap();
            PostMessageW(Some(root), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        },
    )
    .unwrap();
    assert_eq!(result, Some(42));
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn captured_scrollbar_drag_outside_panel_releases_without_selecting() {
    let owner = Owner::new();
    let items = (0..40)
        .map(|id| {
            let mut item = tree().remove(0);
            item.command = id + 100;
            item
        })
        .collect();
    let mut visible = 0;
    let result = run(
        owner.0,
        items,
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        |root| unsafe {
            let mut bounds = RECT::default();
            GetClientRect(root, &mut bounds).unwrap();
            let row = (28 * GetDpiForWindow(owner.0).clamp(48, 768) / 96) as i32;
            visible = (bounds.bottom / row) as usize;
            let bar = crate::native_menu::scrollbar::Bar::new(
                bounds.right,
                bounds.bottom,
                row,
                40,
                visible,
                0,
            )
            .unwrap();
            let post = |message, x: i32, y: i32| {
                let point = LPARAM(((y as u16 as usize) << 16 | x as u16 as usize) as isize);
                PostMessageW(Some(root), message, WPARAM(0), point).unwrap();
            };
            post(WM_LBUTTONDOWN, bar.track.left, bar.thumb.top);
            post(WM_MOUSEMOVE, -20, bounds.bottom + 100);
            post(WM_LBUTTONUP, -20, bounds.bottom + 100);
            // 只有新的完整行点击可执行；拖动松开本身不能结束菜单。
            post(WM_LBUTTONDOWN, 4, 4);
            post(WM_LBUTTONUP, 4, 4);
            PostMessageW(Some(root), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        },
    )
    .unwrap();
    assert_eq!(result, Some(100 + 40 - visible));
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn right_button_selects_but_mismatched_button_release_does_not() {
    let owner = Owner::new();
    for (down, up, expected) in [
        (WM_RBUTTONDOWN, WM_RBUTTONUP, Some(42)),
        (WM_LBUTTONDOWN, WM_RBUTTONUP, None),
        (WM_RBUTTONDOWN, WM_LBUTTONUP, None),
    ] {
        let result = start(owner.0, |root| unsafe {
            let point = LPARAM((4 << 16) | 4);
            PostMessageW(Some(root), down, WPARAM(0), point).unwrap();
            PostMessageW(Some(root), up, WPARAM(0), point).unwrap();
            PostMessageW(Some(root), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        })
        .unwrap();
        assert_eq!(result, expected);
        assert!(unsafe { GetCapture() }.is_invalid());
    }
}

#[test]
fn tray_preparation_failure_destroys_root_without_showing_hidden_owner() {
    let owner = Owner::new();
    let mut root = HWND::default();
    let result = run_prepared(
        owner.0,
        tree(),
        POINT { x: 30, y: 30 },
        unsafe { GetForegroundWindow() },
        |hwnd| {
            root = hwnd;
            assert!(unsafe { IsWindowVisible(hwnd).as_bool() });
            assert!(!unsafe { IsWindowVisible(owner.0).as_bool() });
            Err(Error::new(E_ACCESSDENIED, "synthetic activation refusal"))
        },
        |_| panic!("Rejected activation must not start input"),
    );
    assert!(result.is_err());
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
    assert!(!unsafe { IsWindowVisible(owner.0).as_bool() });
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn prepared_menu_uses_same_queue_and_keeps_business_owner_hidden() {
    let owner = Owner::new();
    let foreground = unsafe { GetForegroundWindow() };
    let mut root = HWND::default();
    let result = run_prepared(
        owner.0,
        tree(),
        POINT { x: 30, y: 30 },
        foreground,
        |hwnd| {
            root = hwnd;
            // 不抢测试机前台；这里只替换激活准入，真实捕获/队列/销毁仍执行。
            assert!(!unsafe { IsWindowVisible(owner.0).as_bool() });
            Ok(foreground)
        },
        |hwnd| unsafe {
            PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x28), LPARAM(0)).unwrap();
            PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x0d), LPARAM(0)).unwrap();
        },
    )
    .unwrap();
    assert_eq!(result, Some(42));
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
    assert!(!unsafe { IsWindowVisible(owner.0).as_bool() });
    assert!(unsafe { GetCapture() }.is_invalid());
}

#[test]
fn live_update_rebuilds_open_submenu_before_next_queued_selection() {
    let owner = Owner::new();
    let mut parent = tree().remove(0);
    parent.children = Some(tree());
    let foreground = unsafe { GetForegroundWindow() };
    let mut updated = false;
    let result = run_updated(
        owner.0,
        vec![parent],
        POINT { x: 30, y: 30 },
        foreground,
        (
            |_| Ok(foreground),
            |root| unsafe {
                for key in [0x28, 0x27, 0x28, 0x0d, 0x1b] {
                    PostMessageW(Some(root), WM_KEYDOWN, WPARAM(key), LPARAM(0)).unwrap();
                }
            },
        ),
        |menu| {
            if menu.submenu_open(0) && !updated {
                let mut replacement = tree();
                replacement[0].command = 77;
                menu.replace_submenu(0, replacement)?;
                updated = true;
            }
            Ok(())
        },
    )
    .unwrap();
    assert!(updated);
    assert_eq!(result, Some(77));
    assert!(unsafe { GetCapture() }.is_invalid());
}
