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
        delete: None,
        preview: None,
        thumbnail: None,
    }
}
fn tree() -> Vec<Item> {
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    vec![parent, item(2)]
}

#[test]
fn thumbnail_only_patch_preserves_gallery_gesture_and_automation_generation() {
    use crate::{image_thumbnail::Thumbnail, native_menu::accessibility::Action};
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    let thumbnail = Thumbnail::from_rgba(1, 1, &[255, 0, 0, 255]).unwrap();
    let other = Thumbnail::from_rgba(1, 1, &[0, 255, 0, 255]).unwrap();
    let mut rows: Vec<_> = (100..164).map(item).collect();
    rows[0].delete = Some(200);
    rows[0].thumbnail = Some(thumbnail.clone());
    menu.replace_gallery(0, rows).unwrap();
    menu.expand(0, 0).unwrap();
    let hwnd = menu.pages[1].panel.handle().unwrap();
    let first = menu.pages[1].first;
    let node = menu.pages[1].panel.accessibility_nodes().remove(0);
    let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
    let (x, y) = (
        (corner.left + corner.right) / 2,
        (corner.top + corner.bottom) / 2,
    );
    menu.event(1, Event::Down(x, y)).unwrap();
    assert!(menu
        .replace_gallery_thumbnails(0, &[0], &[(64, &other)])
        .is_err());
    assert!(
        menu.items(&[0])[0].thumbnail.is_some(),
        "invalid patch must be atomic"
    );
    menu.replace_gallery_thumbnails(0, &[0], &[(1, &other), (63, &thumbnail)])
        .unwrap();
    assert!(menu.items(&[0])[0].thumbnail.is_none());
    assert_eq!(
        menu.items(&[0])[1].thumbnail.as_ref().unwrap().pixels(),
        other.pixels()
    );
    assert_eq!(
        menu.items(&[0])[63].thumbnail.as_ref().unwrap().pixels(),
        thumbnail.pixels()
    );
    assert_eq!(menu.pages[1].panel.handle().unwrap(), hwnd);
    assert_eq!(menu.pages[1].first, first);
    node.enqueue(Action::Focus).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.take_delete(), Some(200));
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
#[test]
fn replacing_favorite_command_keeps_root_but_retires_the_old_pressed_gesture() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(2)], POINT { x: 30, y: 30 }).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    menu.event(0, Event::Down(4, 4)).unwrap();
    menu.replace_action(0, 3, "取消收藏", true).unwrap();
    assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
    menu.event(0, Event::Up(4, 4)).unwrap();
    assert!(menu.take_command().is_none());
    menu.event(0, Event::Down(4, 4)).unwrap();
    menu.event(0, Event::Up(4, 4)).unwrap();
    assert_eq!(menu.take_command(), Some(3));
    assert!(menu.replace_action(0, 4, "收藏", true).is_err());
}

#[test]
fn same_label_replacement_retires_queued_invoke_and_pointer_input() {
    use crate::native_menu::accessibility::{Action, Rejected};
    let owner = Owner::new();
    let row = item(2);
    let label = row.label.clone();
    let mut menu = Cascade::open(owner.0, vec![row], POINT { x: 30, y: 30 }).unwrap();
    let root = menu.pages[0].panel.handle().unwrap();
    let old = menu.pages[0].panel.accessibility_nodes().remove(0);
    menu.replace_action(0, 2, &label, true).unwrap();
    old.enqueue(Action::Invoke).unwrap();
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
        unsafe { SendMessageW(root, message, None, Some(LPARAM(4 | (4 << 16)))) };
    }
    menu.replace_action(0, 3, &label, true).unwrap();
    assert_eq!(old.enqueue(Action::Invoke), Err(Rejected::Unavailable));
    assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
    menu.poll().unwrap();
    assert!(menu.take_command().is_none());
    menu.pages[0].panel.accessibility_nodes()[0]
        .enqueue(Action::Invoke)
        .unwrap();
    menu.poll().unwrap();
    assert_eq!(menu.take_command(), Some(3));
}

#[test]
fn gallery_append_preserves_window_scroll_and_retires_same_named_automation() {
    use crate::native_menu::accessibility::{Action, Rejected};
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    let rows = |count| {
        (0..count)
            .map(|n| {
                let mut row = item(100 + n);
                row.children = Some(vec![item(5000 + n), item(10000 + n)]);
                row
            })
            .collect()
    };
    menu.replace_gallery(0, rows(64)).unwrap();
    menu.expand(0, 0).unwrap();
    menu.set_first(1, usize::MAX).unwrap();
    let hwnd = menu.pages[1].panel.handle().unwrap();
    let first = menu.pages[1].first;
    assert!(menu.gallery_at_end(0));
    let node = menu.pages[1].panel.accessibility_nodes().remove(0);
    node.enqueue(Action::Expand).unwrap();
    // 可见 key/文案/行数完全相同也必须退休，不能靠追加后行数改变使测试偶然通过。
    menu.replace_gallery(0, rows(64)).unwrap();
    assert_eq!(node.enqueue(Action::Expand), Err(Rejected::Unavailable));
    menu.poll().unwrap();
    assert_eq!(menu.pages.len(), 2);
    let node = menu.pages[1].panel.accessibility_nodes().remove(0);
    node.enqueue(Action::Expand).unwrap();
    menu.replace_gallery(0, rows(4096)).unwrap();
    assert_eq!(menu.pages[1].panel.handle().unwrap(), hwnd);
    assert_eq!(menu.pages[1].first, first);
    assert!(!menu.gallery_at_end(0));
    assert_eq!(node.enqueue(Action::Expand), Err(Rejected::Unavailable));
    menu.poll().unwrap();
    assert_eq!(menu.pages.len(), 2);
    let bar = menu.bar(1).expect("full gallery must keep its scrollbar");
    let x = bar.track.left + 1;
    menu.event(1, Event::Down(x, bar.thumb.top)).unwrap();
    menu.event(1, Event::Move(x, i32::MAX)).unwrap();
    menu.event(1, Event::Up(x, i32::MAX)).unwrap();
    assert_eq!(menu.gallery_view(0).unwrap().end, 4096);
    assert!(menu.gallery_at_end(0));
    assert!(menu.replace_gallery(0, rows(4097)).is_err());
    assert_eq!(menu.items(&[0]).len(), 4096);
    assert!(menu.pages[1].panel.accessibility_nodes().len() <= 15);
}

#[test]
fn updating_unopened_library_preserves_current_preview() {
    let owner = Owner::new();
    let mut tree = tree();
    tree[1].children = Some(vec![]);
    let mut menu = Cascade::open(owner.0, tree, POINT { x: 30, y: 30 }).unwrap();
    let mut row = item(100);
    row.preview = Some(crate::native_menu::panel::Preview::Text("preview".into()));
    menu.replace_gallery(0, vec![row]).unwrap();
    menu.expand(0, 0).unwrap();
    menu.event(1, Event::Move(10, 10)).unwrap();
    let hwnd = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    menu.replace_gallery(1, vec![item(200)]).unwrap();
    assert_eq!(menu.preview.as_ref().unwrap().panel.handle().unwrap(), hwnd);
}
