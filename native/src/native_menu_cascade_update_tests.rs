//! 异步替换的真实 HWND 生命周期和预算回滚；不读取系统剪贴板。
use super::*;
use windows::{
    core::w,
    Win32::UI::Input::KeyboardAndMouse::{GetCapture, SetCapture},
};
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("update test"),
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
        label: format!("条目 {command}"),
        enabled: true,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }
}
fn tree() -> Vec<Item> {
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    vec![parent, item(2)]
}
#[test]
fn replacement_preserves_root_capture_and_rejects_old_pressed_row() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    menu.focus(0, Some(0)).unwrap();
    menu.expand(0, 0).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    unsafe {
        SetCapture(root);
    }
    menu.event(1, Event::Down(4, 4)).unwrap();
    menu.pending_surrogate = Some(0xd800);
    menu.replace_submenu(0, vec![item(77), item(78)]).unwrap();
    assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
    assert_eq!(unsafe { GetCapture() }, root);
    assert!(menu.pending_surrogate.is_none());
    assert!(menu.submenu_open(0));
    assert_eq!(menu.pages[1].visible, 2);
    assert!(menu.pages[1].focus.is_none());
    menu.event(1, Event::Up(4, 4)).unwrap();
    assert!(menu.take_command().is_none());
    menu.event(1, Event::Down(4, 4)).unwrap();
    menu.event(1, Event::Up(4, 4)).unwrap();
    assert_eq!(menu.take_command(), Some(77));
    assert!(unsafe { GetCapture() }.is_invalid());
}
#[test]
fn invalid_replacement_keeps_old_tree_and_panel_and_counts_static_items() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    menu.focus(0, Some(0)).unwrap();
    menu.expand(0, 0).unwrap();
    let child = menu.pages[1].panel.handle().unwrap();
    assert!(menu
        .replace_submenu(0, (0..1023).map(item).collect())
        .is_err());
    let mut invalid = item(3);
    invalid.label = "a\0b".into();
    assert!(menu.replace_submenu(0, vec![invalid]).is_err());
    assert_eq!(menu.pages[1].panel.handle().unwrap(), child);
    assert_eq!(menu.items(&[0])[0].command, 42);
    assert!(menu.replace_submenu(1, vec![]).is_err());
    assert!(menu.replace_submenu(usize::MAX, vec![]).is_err());
}
#[test]
fn unopened_update_does_not_open_windows_and_empty_update_cannot_execute_parent() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    menu.replace_submenu(0, vec![]).unwrap();
    assert_eq!(menu.pages.len(), 1);
    menu.key(0x28).unwrap();
    menu.key(0x0d).unwrap();
    assert!(menu.submenu_open(0));
    assert!(menu.take_command().is_none());
    menu.cancel();
    assert!(menu.replace_submenu(0, vec![]).is_err());
}
#[test]
fn handwritten_tree_cannot_bypass_snapshot_budgets() {
    let owner = Owner::new();
    for label in ["a".repeat(4097), "a\0b".into()] {
        let mut entry = item(1);
        entry.label = label;
        assert!(Cascade::open(owner.0, vec![entry], POINT::default()).is_err());
    }
    assert!(Cascade::open(owner.0, (0..1025).map(item).collect(), POINT::default()).is_err());
}
