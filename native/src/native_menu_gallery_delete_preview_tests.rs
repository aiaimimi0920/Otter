//! 删除确认前保留阅读帧，确认后复用 HWND；只使用合成条目和自有窗口。
use super::*;
use crate::native_menu::panel::Preview;

fn entries(first: usize, end: usize) -> Vec<Item> {
    (first..end)
        .map(|n| {
            let mut row = item(100 + n);
            row.delete = Some(200 + n);
            row.preview = Some(Preview::Text(format!("preview-{n}").into()));
            row
        })
        .collect()
}
fn corner(menu: &Cascade, slot: usize) -> (i32, i32) {
    let rect = menu.pages[1]
        .layout
        .delete_rect(slot, menu.content_width(1));
    (rect.left + 2, rect.top + 2)
}
fn assert_preview(menu: &Cascade, hwnd: HWND, bounds: RECT, text: &str) {
    let view = menu.preview.as_ref().expect("preview remains open");
    assert_eq!(view.panel.handle().unwrap(), hwnd);
    assert_eq!(view.bounds, bounds);
    assert!(unsafe { IsWindowVisible(hwnd) }.as_bool());
    let Some(Preview::Text(content)) = view.panel.preview_content() else {
        panic!("text preview")
    };
    assert_eq!(&*content, text);
}
#[test]
fn corner_deletion_keeps_preview_through_pending_and_refresh_until_empty() {
    let (_owner, mut menu) = gallery(3);
    menu.replace_gallery(0, entries(0, 3)).unwrap();
    let (x, y) = corner(&menu, 0);
    menu.event(1, Event::Move(x, y)).unwrap();
    let window = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    let bounds = menu.preview.as_ref().unwrap().bounds;
    for first in 0..3 {
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(x, y)).unwrap();
        assert_eq!(menu.take_delete(), Some(200 + first));
        assert_preview(&menu, window, bounds, &format!("preview-{first}"));
        // 后台尚未确认时保留原画面；抬起不能重放删除。
        menu.event(1, Event::Up(x, y)).unwrap();
        assert!(menu.take_delete().is_none());
        if first < 2 {
            menu.replace_gallery(0, entries(first + 1, 3)).unwrap();
            assert_preview(&menu, window, bounds, &format!("preview-{}", first + 1));
            // 同一位置的后续 hover 不能再次创建或重绘已经更新的预览。
            unsafe { UpdateWindow(window).ok().unwrap() };
            menu.event(1, Event::Move(x, y)).unwrap();
            assert!(!unsafe { GetUpdateRect(window, None, false) }.as_bool());
        }
    }
    let mut empty = item(0);
    empty.enabled = false;
    menu.replace_submenu(0, vec![empty]).unwrap();
    assert!(menu.preview.is_none());
    assert!(!unsafe { IsWindow(Some(window)) }.as_bool());
    assert!(menu.submenu_open(0) && !menu.closed());
}
#[test]
fn deleting_last_scrolled_entry_updates_the_visible_slot_without_reopening_preview() {
    let (_owner, mut menu) = gallery(49);
    menu.replace_gallery(0, entries(0, 49)).unwrap();
    menu.set_first(1, usize::MAX).unwrap();
    let first = menu.pages[1].first;
    let slot = 48 - first;
    let (x, y) = corner(&menu, slot);
    menu.event(1, Event::Move(x, y)).unwrap();
    let window = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    let bounds = menu.preview.as_ref().unwrap().bounds;
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.take_delete(), Some(248));
    assert_preview(&menu, window, bounds, "preview-48");
    menu.replace_gallery(0, entries(0, 48)).unwrap();
    assert!(menu.pages[1].first < first);
    let target = menu.pages[1].first + slot;
    assert_preview(&menu, window, bounds, &format!("preview-{target}"));
    menu.event(1, Event::Move(x, y)).unwrap();
    assert_eq!(menu.preview.as_ref().unwrap().index, target);
    menu.cancel();
    assert!(!unsafe { IsWindow(Some(window)) }.as_bool());
}
#[test]
fn deletion_failure_and_a_removed_unoccupied_slot_release_the_preview() {
    for failed in [false, true] {
        let (_owner, mut menu) = gallery(3);
        menu.replace_gallery(0, entries(0, 3)).unwrap();
        let (x, y) = corner(&menu, 2);
        menu.event(1, Event::Move(x, y)).unwrap();
        let window = menu.preview.as_ref().unwrap().panel.handle().unwrap();
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(x, y)).unwrap();
        assert_eq!(menu.take_delete(), Some(202));
        assert_eq!(
            menu.preview.as_ref().unwrap().panel.handle().unwrap(),
            window
        );
        if failed {
            let mut status = item(0);
            status.enabled = false;
            status.label = "删除失败".into();
            menu.replace_submenu(0, vec![status]).unwrap();
        } else {
            menu.replace_gallery(0, entries(0, 2)).unwrap();
        }
        assert!(menu.preview.is_none());
        assert!(!unsafe { IsWindow(Some(window)) }.as_bool());
        assert!(menu.submenu_open(0) && !menu.closed());
    }
}
