//! 真实面板的滚动与点击隔离；极端坐标验证拖出窗口时仍有界。
use super::*;
use windows::core::w;
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("scroll test"),
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
    (0..40)
        .map(|id| Item {
            command: id,
            label: format!("条目 {id}"),
            enabled: true,
            checked: false,
            radio: false,
            default: false,
            separator: false,
            children: None,
            thumbnail: None,
        })
        .collect()
}
#[test]
fn thumb_drag_clamps_and_release_never_activates_a_row() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    let bar = menu.bar(0).unwrap();
    menu.event(0, Event::Down(bar.track.left, bar.thumb.top))
        .unwrap();
    assert_eq!(menu.dragging(), Some(0));
    menu.event(0, Event::Move(-1000, i32::MAX)).unwrap();
    assert_eq!(menu.pages[0].first, 40 - menu.pages[0].visible);
    menu.event(0, Event::Up(5, 5)).unwrap();
    assert!(menu.dragging().is_none());
    assert!(menu.take_command().is_none());
    let expected = menu.pages[0].first;
    menu.event(0, Event::Down(5, 5)).unwrap();
    menu.event(0, Event::Up(5, 5)).unwrap();
    assert_eq!(menu.take_command(), Some(expected));
}
#[test]
fn track_pages_and_keyboard_ends_drag_without_stale_click() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    let bar = menu.bar(0).unwrap();
    menu.event(0, Event::Down(bar.track.left, bar.track.bottom - 1))
        .unwrap();
    assert_eq!(menu.pages[0].first, menu.pages[0].visible);
    let bar = menu.bar(0).unwrap();
    menu.event(0, Event::Down(bar.track.left, bar.thumb.top))
        .unwrap();
    menu.key(0x28).unwrap();
    assert!(menu.dragging().is_none());
    menu.event(0, Event::Up(5, 5)).unwrap();
    assert!(menu.take_command().is_none());
}
#[test]
fn geometry_is_bounded_for_tiny_viewports_and_extreme_drag() {
    for width in [1, 10, 320] {
        for height in [1, 10, 300] {
            let bar = Bar::new(width, height, 28, 40, 12, 0).unwrap();
            assert!(bar.thumb.top >= 0 && bar.thumb.bottom <= height && bar.track.left >= 0);
            assert!(bar.drag(i32::MIN, 0, i32::MAX) <= 28);
            assert!(bar.drag(i32::MAX, 28, i32::MIN) <= 28);
        }
    }
    assert!(Bar::new(320, 300, 28, 2, 12, 0).is_none());
}
