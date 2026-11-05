//! 实际 DirectWrite 文字命中，不用固定字符宽度替身。
use super::*;
#[path = "surface_text_word_tests.rs"]
mod word;

fn layout(text: &str, whitespace: Whitespace, width: f32) -> Layout {
    Engine::new()
        .unwrap()
        .layout(
            text,
            Style {
                family: "Segoe UI",
                size: 20.,
                weight: 400,
                whitespace,
                align: Align::Start,
                line_height: None,
            },
            width,
            400.,
        )
        .unwrap()
}

#[test]
fn selection_content_uses_rendered_whitespace_and_utf16_not_source_bytes() {
    let value = layout("A\r\n\t  中😀e\u{301}", Whitespace::Normal, 300.);
    assert_eq!(
        value.selected_text(0..value.text_len()).unwrap(),
        "A 中😀e\u{301}"
    );
    assert_eq!(value.selected_text(2..5).unwrap(), "中😀");
    assert!(value.selected_text(3..4).is_err());
    assert!(value.selection_rects(4..5).is_err());
    let pre = layout("A\r\n\t中", Whitespace::PreWrap, 300.);
    assert_eq!(pre.selected_text(0..pre.text_len()).unwrap(), "A\n\t中");
    let clone = value.clone();
    assert!(std::rc::Rc::ptr_eq(&value.text, &clone.text));
}

#[test]
fn selection_hit_preserves_surrogate_and_combining_cluster_edges() {
    let value = layout("😀e\u{301}中", Whitespace::PreWrap, 300.);
    // 每个测试 cluster 左右内部点应落到完整 cluster 边沿，不落在代理项或组合符中间。
    for range in [0..2, 2..4, 4..5] {
        let rect = value.selection_rects(range.clone()).unwrap()[0];
        let y = (rect.top + rect.bottom) / 2.;
        let left = value
            .hit_text([rect.left + (rect.right - rect.left) * 0.1, y])
            .unwrap();
        let right = value
            .hit_text([rect.left + (rect.right - rect.left) * 0.9, y])
            .unwrap();
        assert_eq!(
            left,
            TextHit {
                position: range.start,
                inside: true
            }
        );
        assert_eq!(
            right,
            TextHit {
                position: range.end,
                inside: true
            }
        );
    }
    assert_eq!(
        value.hit_text([-100., 5.]).unwrap(),
        TextHit {
            position: 0,
            inside: false
        }
    );
    assert_eq!(
        value.hit_text([1000., 5.]).unwrap(),
        TextHit {
            position: 5,
            inside: false
        }
    );
}

#[test]
fn selection_geometry_tracks_wrapping_bidi_and_range_bounds() {
    let value = layout("first second third fourth", Whitespace::Normal, 70.);
    let rects = value.selection_rects(0..value.text_len()).unwrap();
    assert!(value.line_count() >= 3);
    assert!(rects.len() >= 3);
    assert!(rects.windows(2).any(|pair| pair[0].top < pair[1].top));
    let bidi = layout("abc אבג xyz", Whitespace::Normal, 300.);
    let rects = bidi.selection_rects(4..7).unwrap();
    assert_eq!(bidi.selected_text(4..7).unwrap(), "אבג");
    let rect = rects[0];
    let left = bidi
        .hit_text([rect.left + 0.1, (rect.top + rect.bottom) / 2.])
        .unwrap();
    let right = bidi
        .hit_text([rect.right - 0.1, (rect.top + rect.bottom) / 2.])
        .unwrap();
    assert_eq!(left.position, 7);
    assert_eq!(right.position, 4);
    assert!(value.selection_rects(0..10000).is_err());
    assert!(value
        .selected_text(std::ops::Range { start: 2, end: 1 })
        .is_err());
    assert!(value.selection_rects(0..0).unwrap().is_empty());
    assert!(value.hit_text([f32::NAN, 0.]).is_err());
    assert!(value.hit_text([0., f32::INFINITY]).is_err());
    let empty = layout("", Whitespace::Normal, 100.);
    assert_eq!(empty.selected_text(0..0).unwrap(), "");
    assert_eq!(empty.hit_text([0., 0.]).unwrap().position, 0);
}

#[test]
fn text_selection_step_uses_dwrite_clusters_and_clamps_edges() {
    let value = layout("😀e\u{301}中\r\nZ", Whitespace::PreWrap, 35.);
    let expected = [0, 2, 4, 5, 6, 7];
    for pair in expected.windows(2) {
        assert_eq!(value.step_text(pair[0], true).unwrap(), pair[1]);
        assert_eq!(value.step_text(pair[1], false).unwrap(), pair[0]);
    }
    assert_eq!(value.step_text(0, false).unwrap(), 0);
    assert_eq!(value.step_text(7, true).unwrap(), 7);
    assert!(value.step_text(1, true).is_err());
    assert!(value.step_text(8, false).is_err());
    let empty = layout("", Whitespace::Normal, 100.);
    assert_eq!(empty.step_text(0, false).unwrap(), 0);
    assert_eq!(empty.step_text(0, true).unwrap(), 0);
}

#[test]
fn text_selection_vertical_retains_column_across_short_line_and_clamps_document_edges() {
    let value = layout("WWWW\nI\nWWWW", Whitespace::PreWrap, 300.);
    let (short, column) = value.vertical_text(3, true, None).unwrap();
    assert_eq!(short, 6);
    let (long, same) = value.vertical_text(short, true, Some(column)).unwrap();
    assert_eq!(long, 10);
    assert_eq!(column[0], same[0]);
    let (short, column) = value.vertical_text(long, false, Some(same)).unwrap();
    assert_eq!(short, 6);
    assert_eq!(
        value.vertical_text(short, false, Some(column)).unwrap().0,
        3
    );
    assert_eq!(value.vertical_text(3, false, None).unwrap().0, 3);
    assert_eq!(value.vertical_text(10, true, None).unwrap().0, 10);
    assert!(value.vertical_text(12, true, None).is_err());
    assert!(value.vertical_text(0, true, Some([f32::NAN, 0.])).is_err());
    assert_eq!(
        layout("", Whitespace::Normal, 100.)
            .vertical_text(0, true, None)
            .unwrap(),
        (0, [0., 0.])
    );
}

#[test]
fn text_selection_vertical_wrapped_unicode_uses_cluster_hit_not_character_count() {
    let value = layout(
        "😀e\u{301}中😀e\u{301}中😀e\u{301}中",
        Whitespace::PreWrap,
        45.,
    );
    assert!(value.line_count() >= 3);
    let mut position = 0;
    let mut column = None;
    for _ in 0..value.line_count() {
        let (next, x) = value.vertical_text(position, true, column).unwrap();
        assert!(next >= position);
        assert!(![1, 3, 6, 8, 11, 13].contains(&next));
        value.selected_text(0..next).unwrap();
        position = next;
        column = Some(x);
    }
    assert!(position > 0);
}

#[test]
fn text_selection_vertical_retains_soft_wrap_affinity_instead_of_skipping_rows() {
    let value = layout(
        "one two three four five six seven eight",
        Whitespace::Normal,
        70.,
    );
    let line_height = value.metrics.height / value.line_count() as f32;
    let mut position = 0;
    let mut point = [300., 1.];
    for line in 1..value.line_count() {
        let (next, target) = value.vertical_text(position, true, Some(point)).unwrap();
        assert!(target[1] >= line as f32 * line_height);
        assert!(target[1] < (line + 1) as f32 * line_height);
        assert!(next >= position);
        position = next;
        point = target;
    }
    assert_eq!(
        value.vertical_text(position, true, Some(point)).unwrap(),
        (position, point)
    );
}
#[test]
fn text_selection_vertical_step_reports_soft_wrap_motion_even_at_same_offset() {
    let value = layout("one two three four five six", Whitespace::Normal, 70.);
    let (edge, row) = value.line_edge(0, true, None).unwrap();
    let (next, point, moved) = value
        .vertical_text_step(edge, true, Some([0., row[1]]))
        .unwrap();
    assert_eq!(next, edge);
    assert!(moved);
    assert!(point[1] > row[1]);
}
#[test]
fn text_selection_vertical_step_handles_empty_and_trailing_newline_rows() {
    let empty = layout("", Whitespace::Pre, 70.);
    assert_eq!(
        empty.vertical_text_step(0, true, None).unwrap(),
        (0, [0.; 2], false)
    );
    for text in ["A\n", "A\r\n"] {
        let value = layout(text, Whitespace::Pre, 70.);
        let (end, point, moved) = value.vertical_text_step(0, true, None).unwrap();
        assert_eq!(end, value.text_len());
        assert!(moved);
        assert!(!value.vertical_text_step(end, true, Some(point)).unwrap().2);
        let (start, _, moved) = value.vertical_text_step(end, false, Some(point)).unwrap();
        assert_eq!(start, 0);
        assert!(moved);
    }
}
