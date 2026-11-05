//! 真实级联 HWND 与导航消息；不修改业务文档或读取剪贴板。
#[path = "native_menu_uia_client_tests.rs"]
mod uia_client;
use super::*;
use windows::core::w;
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("cascade test"),
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
fn item(id: usize) -> Item {
    Item {
        command: id,
        label: format!("命令 {id}"),
        enabled: true,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }
}

#[test]
fn hover_opens_real_child_and_keyboard_returns_command_after_cleanup() {
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.event(0, Event::Move(10, 10)).unwrap();
    assert_eq!(menu.pages.len(), 2);
    let handles: Vec<_> = menu
        .pages
        .iter()
        .map(|page| page.panel.handle().unwrap())
        .collect();
    menu.key(0x28).unwrap();
    menu.key(0x0d).unwrap();
    assert!(menu.closed());
    assert_eq!(menu.take_command(), Some(42));
    assert!(menu.take_command().is_none());
    for hwnd in handles {
        assert!(!unsafe { IsWindow(Some(hwnd)).as_bool() });
    }
}
#[test]
fn keyboard_skips_disabled_separator_and_reveals_offscreen_rows() {
    let owner = Owner::new();
    let mut items: Vec<_> = (0..40).map(item).collect();
    items[0].enabled = false;
    items[1].separator = true;
    let mut menu = Cascade::open(owner.0, items, POINT { x: 30, y: 30 }).unwrap();
    menu.key(0x28).unwrap();
    assert_eq!(menu.pages[0].focus, Some(2));
    for _ in 0..20 {
        menu.key(0x28).unwrap();
    }
    assert!(menu.pages[0].first > 0);
    assert_eq!(menu.pages[0].focus, Some(22));
    menu.key(0x0d).unwrap();
    assert_eq!(menu.take_command(), Some(22));
}
#[test]
fn mismatched_click_and_owner_destruction_never_return_command() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(1), item(2)], POINT { x: 30, y: 30 }).unwrap();
    menu.event(0, Event::Down(10, 1)).unwrap();
    menu.event(0, Event::Up(10, menu.row + 1)).unwrap();
    assert!(menu.take_command().is_none());
    menu.key(0x28).unwrap();
    drop(owner);
    menu.key(0x0d).unwrap();
    menu.poll().unwrap();
    assert!(menu.closed());
    assert!(menu.take_command().is_none());
}

#[test]
fn right_focuses_first_enabled_child_left_restores_parent_and_escape_closes() {
    let owner = Owner::new();
    let mut disabled = item(40);
    disabled.enabled = false;
    let mut parent = item(1);
    parent.children = Some(vec![disabled, item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.key(0x28).unwrap();
    menu.key(0x27).unwrap();
    assert_eq!(menu.pages[1].focus, Some(1));
    menu.key(0x25).unwrap();
    assert_eq!(menu.pages.len(), 1);
    assert_eq!(menu.pages[0].focus, Some(0));
    menu.key(0x1b).unwrap();
    assert!(menu.closed());
    assert!(menu.take_command().is_none());
}

#[test]
fn accessibility_requests_use_existing_cascade_and_revoke_after_close() {
    use crate::native_menu::accessibility::Action;
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    let parent = menu.pages[0].panel.accessibility_nodes().remove(0);
    parent.enqueue(Action::Expand).unwrap();
    menu.poll().unwrap();
    assert_eq!(menu.pages.len(), 2);
    assert!(menu.take_command().is_none());
    let leaf = menu.pages[1].panel.accessibility_nodes().remove(0);
    leaf.enqueue(Action::Invoke).unwrap();
    assert!(menu.take_command().is_none()); // 入队不是业务已执行。
    menu.poll().unwrap();
    assert!(menu.closed());
    assert_eq!(menu.take_command(), Some(42));
    assert!(leaf.read().is_none());
}

#[test]
fn accessibility_visible_slot_cannot_target_a_different_row_after_scroll() {
    use crate::native_menu::accessibility::{Action, Rejected};
    let owner = Owner::new();
    let mut menu =
        Cascade::open(owner.0, (0..40).map(item).collect(), POINT { x: 30, y: 30 }).unwrap();
    let stale = menu.pages[0].panel.accessibility_nodes().remove(0);
    stale.enqueue(Action::Invoke).unwrap();
    for _ in 0..20 {
        menu.key(0x28).unwrap();
    }
    assert!(menu.pages[0].first > 0);
    assert!(stale.read().is_none());
    assert_eq!(stale.enqueue(Action::Invoke), Err(Rejected::Unavailable));
    menu.poll().unwrap();
    assert!(menu.take_command().is_none());
    let first = menu.pages[0].first;
    menu.pages[0].panel.accessibility_nodes()[0]
        .enqueue(Action::Invoke)
        .unwrap();
    menu.poll().unwrap();
    assert_eq!(menu.take_command(), Some(first));
}

#[test]
fn accessibility_collapse_revokes_child_without_executing_parent() {
    use crate::native_menu::accessibility::{Action, Rejected};
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.pages[0].panel.accessibility_nodes()[0]
        .enqueue(Action::Focus)
        .unwrap();
    menu.poll().unwrap();
    assert_eq!(menu.pages[0].focus, Some(0));
    menu.pages[0].panel.accessibility_nodes()[0]
        .enqueue(Action::Expand)
        .unwrap();
    menu.poll().unwrap();
    let child = menu.pages[1].panel.accessibility_nodes().remove(0);
    menu.pages[0].panel.accessibility_nodes()[0]
        .enqueue(Action::Collapse)
        .unwrap();
    menu.poll().unwrap();
    assert_eq!(menu.pages.len(), 1);
    assert!(menu.take_command().is_none());
    assert_eq!(child.enqueue(Action::Invoke), Err(Rejected::Unavailable));
}
