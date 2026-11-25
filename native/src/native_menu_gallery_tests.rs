//! 三列卡片的真实面板命中/滚动和替换回归；只操作自有 HWND，不读真实库或剪贴板。
use super::*;
#[path = "native_menu_gallery_delete_preview_tests.rs"]
mod delete_preview;
#[path = "native_menu_gallery_input_tests.rs"]
mod input;

fn gallery(count: usize) -> (Owner, Cascade) {
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(Vec::new());
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    menu.replace_gallery(0, (0..count).map(|n| item(100 + n)).collect())
        .unwrap();
    menu.expand(0, 0).unwrap();
    (owner, menu)
}
fn center(menu: &Cascade, slot: usize) -> (i32, i32) {
    let rect = menu.pages[1].layout.rect(slot, menu.content_width(1));
    ((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2)
}
#[test]
fn gallery_columns_hit_distinct_items_and_a_cross_cell_gesture_does_not_execute() {
    for selected in 0..3 {
        let (_owner, mut menu) = gallery(7);
        assert_eq!(menu.pages[1].layout.columns, 3);
        let (x, y) = center(&menu, selected);
        let (other_x, other_y) = center(&menu, (selected + 1) % 3);
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(other_x, other_y)).unwrap();
        assert!(menu.take_command().is_none());
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(x, y)).unwrap();
        assert_eq!(menu.take_command(), Some(100 + selected));
    }
}
#[test]
fn gallery_scroll_stays_row_aligned_and_old_nodes_and_pressed_cells_are_retired() {
    let (_owner, mut menu) = gallery(64);
    let old_node = menu.pages[1].panel.accessibility_nodes().remove(0);
    let (x, y) = center(&menu, 0);
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Wheel(-120)).unwrap();
    assert!(menu.pages[1].first > 0);
    assert_eq!(menu.pages[1].first % 3, 0);
    assert!(old_node.read().is_none());
    menu.event(1, Event::Up(x, y)).unwrap();
    assert!(menu.take_command().is_none());
    menu.set_first(1, usize::MAX).unwrap();
    assert_eq!(
        menu.pages[1].first,
        menu.pages[1].layout.max_first(64, menu.pages[1].visible)
    );
    menu.focus(1, Some(63)).unwrap();
    assert!(menu.pages[1].first <= 63 && 63 < menu.pages[1].first + menu.pages[1].visible);
    menu.replace_gallery(0, vec![item(200)]).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert!(menu.take_command().is_none());
}
#[test]
fn hovering_a_card_does_not_cover_other_images_with_its_actions() {
    let (_owner, mut menu) = gallery(3);
    let mut tile = item(100);
    tile.children = Some(vec![item(101), item(102)]);
    menu.replace_gallery(0, vec![tile, item(103), item(104)])
        .unwrap();
    let (x, y) = center(&menu, 0);
    menu.event(1, Event::Move(x, y)).unwrap();
    assert_eq!(menu.pages.len(), 2);
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.pages.len(), 3);
    assert_eq!(menu.pages[2].layout.columns, 1);
    menu.key(0x28).unwrap();
    menu.key(0x0d).unwrap();
    assert_eq!(menu.take_command(), Some(101));
}
#[test]
fn gallery_accessibility_uses_the_same_columns_and_excludes_the_scrollbar() {
    let rows: Vec<_> = (0..6)
        .map(|key| Row {
            key,
            deletable: false,
            thumbnail: None,
            label: key.to_string(),
            enabled: true,
            checked: false,
            submenu: false,
            separator: false,
            focused: false,
            radio: false,
            default: false,
        })
        .collect();
    let owner = crate::native_menu::accessibility::Owner::new(&rows);
    owner.set_columns(3);
    owner.set_content_width(241);
    let view = owner.view();
    let layout = crate::native_menu::layout::Layout {
        columns: 3,
        row: 80,
    };
    for slot in 0..6 {
        let node = view.node(slot).unwrap();
        let rect = layout.rect(slot, 241);
        assert_eq!(node.item_rect(248, 80), Some(rect));
        for (x, y) in [(rect.left, rect.top), (rect.right - 1, rect.bottom - 1)] {
            assert_eq!(
                view.node_at_point(x, y, 248, 80).unwrap().runtime_id(),
                node.runtime_id()
            );
        }
    }
    for (x, y) in [(80, 20), (120, 80), (242, 20), (40, 165)] {
        assert!(view.node_at_point(x, y, 248, 80).is_none());
    }
}
