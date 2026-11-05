//! 移动真实面板 HWND 后复核路由；不冒充物理鼠标或多显示器 DPI 验收。
use super::*;
use windows::core::w;

struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("geometry fixture"),
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
fn item(command: usize) -> Item {
    Item {
        command,
        label: format!("item {command}"),
        enabled: true,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }
}
fn relocate(menu: &Cascade, level: usize) -> RECT {
    let page = &menu.pages[level];
    let bounds = RECT {
        left: page.bounds.left + 43,
        right: page.bounds.right + 43,
        top: page.bounds.top + 57,
        bottom: page.bounds.bottom + 57,
    };
    unsafe {
        SetWindowPos(
            page.panel.handle().unwrap(),
            None,
            bounds.left,
            bounds.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    bounds
}
fn pointer(menu: &mut Cascade, hwnd: HWND, message: u32) {
    menu.pointer(&MSG {
        hwnd,
        message,
        lParam: LPARAM(5 | (5 << 16)),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn relocated_child_routes_click_to_actual_hwnd_and_rejects_old_press() {
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.key(0x28).unwrap();
    menu.key(0x27).unwrap();
    let child = menu.pages[1].panel.handle().unwrap();
    pointer(&mut menu, child, WM_LBUTTONDOWN);
    assert_eq!(menu.pages[1].pressed, Some(0));
    let expected = relocate(&menu, 1);
    pointer(&mut menu, child, WM_LBUTTONUP);
    assert_eq!(menu.pages[1].bounds, expected);
    assert!(!menu.closed());
    assert!(menu.take_command().is_none());
    pointer(&mut menu, child, WM_LBUTTONDOWN);
    pointer(&mut menu, child, WM_LBUTTONUP);
    assert_eq!(menu.take_command(), Some(42));
    assert!(menu.closed());
}

#[test]
fn relocation_invalidates_buffered_pointer_input_but_preserves_keyboard() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(42)], POINT { x: 30, y: 30 }).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    unsafe {
        SendMessageW(root, WM_LBUTTONDOWN, None, Some(LPARAM(5 | (5 << 16))));
        SendMessageW(root, WM_LBUTTONUP, None, Some(LPARAM(5 | (5 << 16))));
        SendMessageW(root, WM_KEYDOWN, Some(WPARAM(0x28)), None);
    }
    let expected = relocate(&menu, 0);
    menu.poll().unwrap();
    assert_eq!(menu.pages[0].bounds, expected);
    assert_eq!(menu.pages[0].focus, Some(0));
    assert!(menu.take_command().is_none());
    menu.key(0x0d).unwrap();
    assert_eq!(menu.take_command(), Some(42));
}

#[test]
fn external_resize_cancels_instead_of_using_stale_row_geometry() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(42)], POINT { x: 30, y: 30 }).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    unsafe {
        SetWindowPos(
            root,
            None,
            0,
            0,
            100,
            100,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    menu.key(0x28).unwrap();
    assert!(menu.closed());
    assert!(menu.take_command().is_none());
}
