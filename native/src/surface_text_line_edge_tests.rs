//! 行长度包含换行，End 排除换行但保留尾随空白；直接使用真实排版。
use super::*;
fn layout(text: &str, width: f32) -> Layout {
    Engine::new()
        .unwrap()
        .layout(
            text,
            Style {
                family: "Segoe UI",
                size: 20.,
                weight: 400,
                whitespace: Whitespace::PreWrap,
                align: Align::Start,
                line_height: None,
            },
            width,
            400.,
        )
        .unwrap()
}
#[test]
fn text_selection_line_edge_excludes_newline_and_preserves_spaces_and_unicode() {
    let text = layout("😀e\u{301}  \r\n中Z", 300.);
    assert_eq!(text.line_edge(2, false, None).unwrap().0, 0);
    let (end, point) = text.line_edge(2, true, None).unwrap();
    assert_eq!(end, 6);
    assert_eq!(text.selected_text(0..end).unwrap(), "😀e\u{301}  ");
    assert_eq!(text.line_edge(end, true, Some(point)).unwrap().0, end);
    assert_eq!(text.line_edge(8, false, None).unwrap().0, 7);
    assert_eq!(text.line_edge(8, true, None).unwrap().0, 9);
    assert!(text.line_edge(1, true, None).is_err());
    assert!(text.line_edge(10, true, None).is_err());
    assert!(text.line_edge(0, true, Some([0., f32::NAN])).is_err());
    let endings = layout("😀\ne\u{301}", 300.);
    let (emoji_end, emoji_point) = endings.line_edge(0, true, None).unwrap();
    assert_eq!(emoji_end, 2);
    assert_eq!(emoji_point[0], endings.cluster_rect(0).unwrap().right);
    let (combining_end, combining_point) = endings.line_edge(3, true, None).unwrap();
    assert_eq!(combining_end, 5);
    assert_eq!(combining_point[0], endings.cluster_rect(3).unwrap().right);
}
#[test]
fn text_selection_line_edge_keeps_soft_wrap_affinity_across_home_end_and_down() {
    let text = layout("one two three four five six seven eight", 70.);
    assert!(text.line_count() > 2);
    let (end, point) = text.line_edge(0, true, None).unwrap();
    assert!(end > 0 && end < text.text_len());
    let (home, home_point) = text.line_edge(end, false, Some(point)).unwrap();
    assert_eq!(home, 0);
    assert_eq!(home_point[1], point[1]);
    assert_eq!(text.line_edge(end, true, Some(point)).unwrap().0, end);
    let (next, next_point) = text.vertical_text(end, true, Some(point)).unwrap();
    assert_eq!(
        text.line_edge(next, false, Some(next_point)).unwrap().0,
        end
    );
}
#[test]
fn text_selection_line_edge_handles_empty_middle_and_terminal_lines() {
    let text = layout("A\n\nB\n", 300.);
    for (position, edge) in [(2, 2), (5, 5)] {
        for end in [false, true] {
            assert_eq!(text.line_edge(position, end, None).unwrap().0, edge);
        }
    }
    let empty = layout("", 300.);
    assert_eq!(empty.line_edge(0, true, None).unwrap(), (0, [0.; 2]));
}
