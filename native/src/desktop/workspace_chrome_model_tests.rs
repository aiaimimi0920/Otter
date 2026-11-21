use super::*;
fn node() -> Node {
    Node {
        border_alpha: 255,
        composite_frame: false,
        endpoint: 0,
        hwnd: 1,
        marker: 2,
        rect: Rect {
            left: -100,
            top: -200,
            right: 100,
            bottom: 200,
        },
        dpi: 96,
        minified: false,
        expanded: false,
        hide_collapsed_endpoints: false,
        interactive: true,
        protected: true,
        inputs: 2,
        outputs: 1,
        input_visibility: std::rc::Rc::from([]),
        output_visibility: std::rc::Rc::from([]),
    }
}
#[test]
fn hidden_endpoints_keep_their_normal_rail_slots_without_becoming_visible_or_hittable() {
    let mut unit = node();
    unit.inputs = 4;
    unit.input_visibility = std::rc::Rc::from([false, true, false, true]);
    assert_eq!(unit.point(Side::Input, 0), None);
    assert_eq!(unit.point(Side::Input, 2), None);
    assert_eq!(unit.point(Side::Input, 1), Some(Point { x: -102, y: -128 }));
    assert_eq!(unit.point(Side::Input, 3), Some(Point { x: -102, y: -56 }));
    unit.input_visibility = std::rc::Rc::from([false; 4]);
    assert!((0..4).all(|i| unit.point(Side::Input, i).is_none()));
    assert!(unit.point(Side::Output, 0).is_some());
}
#[test]
fn expanded_rails_reveal_hidden_rows_at_stable_indices_and_compact_rows_still_obey_visibility() {
    let mut unit = node();
    unit.inputs = 4;
    unit.input_visibility = std::rc::Rc::from([false, true, false, true]);
    unit.expanded = true;
    for index in 0..4 {
        assert_eq!(
            unit.point(Side::Input, index).unwrap().y,
            -164 + index as i32 * 36
        );
    }
    unit.expanded = false;
    assert_eq!(
        unit.point_revealed(Side::Input, 0, |i| i == 0).unwrap().y,
        -164
    );
    assert_eq!(
        unit.point_revealed(Side::Input, 1, |i| i == 0).unwrap().y,
        -128
    );
    unit.minified = true;
    unit.expanded = true;
    assert!(unit.point_revealed(Side::Input, 0, |_| true).is_none());
    assert!(unit.point(Side::Input, 1).is_some());
}
#[test]
fn hidden_normal_slots_keep_visible_anchor_positions_across_dpi_and_tab_edges() {
    for dpi in [96, 120, 144, 192, 768] {
        let mut unit = node();
        unit.dpi = dpi;
        unit.inputs = 4;
        let original = [unit.point(Side::Input, 1), unit.point(Side::Input, 3)];
        unit.input_visibility = std::rc::Rc::from([false, true, false, true]);
        assert_eq!(
            [unit.point(Side::Input, 1), unit.point(Side::Input, 3)],
            original
        );
        unit.expanded = true;
        assert_eq!(
            [unit.point(Side::Input, 1), unit.point(Side::Input, 3)],
            original
        );
        assert!(unit.point(Side::Input, 0).is_some());
        assert!(unit.point_revealed(Side::Input, 0, |_| false).is_none());
        unit.minified = true;
        assert_eq!(unit.point(Side::Input, 1).unwrap().y, -100);
        assert_eq!(unit.point(Side::Input, 3).unwrap().y, 100);
    }
}
#[test]
fn three_modes_preserve_border_body_but_only_all_shows_collapsed_endpoints() {
    let unit = node();
    let mut mode = Mode::default();
    for (expected, frame, ports) in [
        (Mode::All, true, true),
        (Mode::Border, true, false),
        (Mode::None, false, false),
        (Mode::All, true, true),
    ] {
        assert_eq!(mode, expected);
        assert_eq!(unit.frame(mode), frame);
        assert_eq!(unit.ports(mode, false), ports);
        mode = mode.next();
    }
    assert!(!unit.ports(Mode::All, true));
}
#[test]
fn tab_expanded_overrides_clean_mode_and_hide_collapsed_but_not_minification() {
    let mut unit = node();
    unit.expanded = true;
    for mode in [Mode::All, Mode::Border, Mode::None] {
        assert!(unit.ports(mode, true));
    }
    unit.minified = true;
    assert!(unit.frame(Mode::All));
    assert!(unit.frame(Mode::Border));
    assert!(!unit.frame(Mode::None));
    assert!(!unit.ports(Mode::None, false));
    assert!(unit.ports(Mode::All, false));
    assert_eq!(unit.radius(), 4);
}
#[test]
fn normal_rails_use_main_top_22_height_28_gap_8_and_outline_outset() {
    let unit = node();
    assert_eq!(unit.point(Side::Input, 0), Some(Point { x: -102, y: -164 }));
    assert_eq!(unit.point(Side::Input, 1), Some(Point { x: -102, y: -128 }));
    assert_eq!(unit.point(Side::Output, 0), Some(Point { x: 102, y: -164 }));
    assert_eq!(unit.point(Side::Output, 1), None);
    assert_eq!(unit.radius(), 14);
}
#[test]
fn link_body_and_dashed_expanded_overlay_have_distinct_mode_rules() {
    let mut from = node();
    let mut to = node();
    assert_eq!(edge_style(Mode::Border, &from, &to), (true, false));
    from.expanded = true;
    assert_eq!(edge_style(Mode::Border, &from, &to), (true, true));
    assert_eq!(edge_style(Mode::None, &from, &to), (false, false));
    to.expanded = true;
    assert_eq!(edge_style(Mode::None, &from, &to), (false, true));
}
#[test]
fn five_css_pixel_threshold_is_scaled_and_extreme_deltas_do_not_overflow() {
    let start = Point::default();
    assert!(!moved(start, Point { x: 4, y: 0 }, 96));
    assert!(moved(start, Point { x: 3, y: 4 }, 96));
    assert!(!moved(start, Point { x: 9, y: 0 }, 192));
    assert!(moved(start, Point { x: 10, y: 0 }, 192));
    assert!(moved(
        Point {
            x: i32::MIN,
            y: i32::MIN
        },
        Point {
            x: i32::MAX,
            y: i32::MAX
        },
        768
    ));
}
#[test]
fn degenerate_rectangles_never_show_or_hit_and_compact_markers_remain_inside_the_image() {
    let mut unit = node();
    unit.inputs = 256;
    unit.minified = true;
    for index in 0..256 {
        let point = unit.point(Side::Input, index).unwrap();
        assert!((-200..200).contains(&point.y));
    }
    unit.rect.right = unit.rect.left;
    assert!(!unit.frame(Mode::All));
    assert!(!unit.ports(Mode::All, false));
    assert!(unit.point(Side::Input, 0).is_none());
}
