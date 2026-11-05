//! 解析与真实级联窗口测试；命令仍由调用者在目标校验后执行。
use super::*;
use windows::core::w;
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("mnemonic test"),
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
fn item(id: usize, label: &str) -> Item {
    Item {
        command: id,
        label: label.into(),
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
fn escapes_shortcuts_unicode_and_invalid_utf16_are_explicit() {
    assert_eq!(mnemonic("鱼&&薯条(&C)\tCtrl+&X"), Some('C'));
    assert_eq!(mnemonic("&&literal\t&X"), None);
    assert_eq!(mnemonic("trailing&"), None);
    assert_eq!(mnemonic("&字"), Some('字'));
    let mut pending = None;
    assert_eq!(character(&mut pending, 0xdc00), None);
    assert_eq!(character(&mut pending, 0xd83d), None);
    assert_eq!(character(&mut pending, 0xde00), Some('😀'));
    character(&mut pending, 0xd800);
    assert_eq!(character(&mut pending, b'A' as u16), Some('A'));
    assert_eq!(character(&mut pending, 0xdc00), None);
}
#[test]
fn duplicate_mnemonics_cycle_skip_disabled_and_reveal_then_enter() {
    let owner = Owner::new();
    let mut items: Vec<_> = (0..20).map(|id| item(id, "normal")).collect();
    items[0] = item(0, "&A disabled");
    items[0].enabled = false;
    items[1] = item(1, "&A first");
    items[19] = item(19, "&a last");
    let mut menu = Cascade::open(owner.0, items, POINT { x: 30, y: 30 }).unwrap();
    menu.character(b'a' as u16).unwrap();
    assert_eq!(menu.pages[0].focus, Some(1));
    menu.character(b'A' as u16).unwrap();
    assert_eq!(menu.pages[0].focus, Some(19));
    assert!(menu.pages[0].first > 0);
    assert!(menu.take_command().is_none());
    menu.key(0x0d).unwrap();
    assert_eq!(menu.take_command(), Some(19));
}
#[test]
fn unique_submenu_focuses_child_and_owner_loss_blocks_characters() {
    let owner = Owner::new();
    let mut parent = item(1, "&Tools");
    parent.children = Some(vec![item(42, "&执行")]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.character(b't' as u16).unwrap();
    assert_eq!(menu.pages.len(), 2);
    assert_eq!(menu.pages[1].focus, Some(0));
    menu.character('执' as u16).unwrap();
    assert_eq!(menu.take_command(), Some(42));
    let mut menu = Cascade::open(owner.0, vec![item(7, "&A")], POINT { x: 30, y: 30 }).unwrap();
    drop(owner);
    menu.character(b'A' as u16).unwrap();
    assert!(menu.closed());
    assert!(menu.take_command().is_none());
}

#[test]
fn surrogate_pairs_cannot_cross_navigation_but_complete_pairs_execute() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(77, "&😀")], POINT { x: 30, y: 30 }).unwrap();
    menu.character(0xd83d).unwrap();
    menu.key(0x28).unwrap();
    menu.character(0xde00).unwrap();
    assert!(!menu.closed());
    assert!(menu.take_command().is_none());
    menu.character(0xd83d).unwrap();
    menu.character(0xde00).unwrap();
    assert_eq!(menu.take_command(), Some(77));
}
