use super::*;
fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
    RECT {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

#[test]
fn anchor_alignment_left_flip_and_vertical_clamp_share_hit_coordinates() {
    let work = rect(-1920, -200, 0, 880);
    let list = List::place(rect(-80, 800, 0, 830), work, 300, 30, 25).unwrap();
    assert_eq!(list.bounds, rect(-380, 640, -80, 880));
    assert_eq!(list.hit(POINT { x: -300, y: 641 }), Some(Hit::Row(0)));
    assert_eq!(list.hit(POINT { x: -300, y: 639 }), None);
    let aligned = List::place(rect(-1800, 0, -1600, 30), work, 300, 30, 2).unwrap();
    assert_eq!(aligned.bounds, rect(-1600, 0, -1300, 60));
}

#[test]
fn wheel_and_thumb_drag_select_the_scrolled_row_and_clamp() {
    let mut list = List::place(rect(0, 0, 100, 30), rect(0, 0, 1000, 800), 300, 30, 25).unwrap();
    assert!(list.scroll(60));
    assert_eq!(list.hit(POINT { x: 110, y: 1 }), Some(Hit::Row(2)));
    assert!(list.row_rect(0).is_none());
    let thumb = list.thumb().unwrap();
    assert!(list.press(POINT {
        x: thumb.left,
        y: thumb.top
    }));
    assert!(list.drag(POINT {
        x: thumb.left,
        y: i32::MAX
    }));
    assert_eq!(list.offset(), 510);
    assert_eq!(list.hit(POINT { x: 110, y: 239 }), Some(Hit::Row(24)));
    list.cancel_drag();
    assert!(!list.drag(POINT { x: 0, y: i32::MIN }));
    assert!(list.scroll(i32::MIN));
    assert_eq!(list.offset(), 0);
}

#[test]
fn empty_tiny_and_invalid_viewports_have_no_invented_rows() {
    let list = List::place(rect(0, 0, 20, 20), rect(0, 0, 10, 5), 300, 30, 0).unwrap();
    assert_eq!(list.bounds, rect(0, 0, 10, 5));
    assert_eq!(list.hit(POINT { x: 1, y: 1 }), None);
    assert!(list.thumb().is_none());
    assert!(List::place(rect(0, 0, 0, 0), rect(0, 0, 0, 0), 300, 30, 1).is_none());
    assert!(List::place(rect(0, 0, 0, 0), rect(0, 0, 100, 100), 300, 0, 1).is_none());
    assert!(List::place(rect(0, 0, 0, 0), rect(0, 0, 100, 100), 300, 30, 26).is_none());
}

#[test]
fn asynchronous_replacement_repositions_and_does_not_inherit_drag() {
    let anchor = rect(800, 700, 900, 730);
    let work = rect(0, 0, 1000, 800);
    let mut old = List::place(anchor, work, 300, 30, 25).unwrap();
    let thumb = old.thumb().unwrap();
    old.press(POINT {
        x: thumb.left,
        y: thumb.top,
    });
    let mut new = List::place(anchor, work, 300, 30, 2).unwrap();
    assert_eq!(new.bounds.top, 700);
    assert!(!new.drag(POINT { x: 900, y: 799 }));
    assert_eq!(new.hit(POINT { x: 600, y: 731 }), Some(Hit::Row(1)));
    assert!(new
        .hit(POINT {
            x: 600,
            y: old.bounds.top
        })
        .is_none());
}
