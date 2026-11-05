//! 实际 DirectWrite/滚动投影；验证局部端点而非长文本框整体显露。
use super::*;
use crate::surface_text::{Align, Whitespace};
fn owner(json: &str) -> Presentation {
    Presentation::new(
        Scene::parse(json.as_bytes()).unwrap(),
        Style {
            family: "Segoe UI",
            size: 20.,
            weight: 400,
            whitespace: Whitespace::PreWrap,
            align: Align::Start,
            line_height: None,
        },
        Context {
            axis: 200.,
            viewport: [200., 100.],
            container: [200., 100.],
            root_font: 16.,
            font: 20.,
            fraction: None,
        },
        [200., 100.],
    )
    .unwrap()
}
fn origin(p: &Presentation, id: &str) -> [f32; 2] {
    p.geometry
        .nodes()
        .iter()
        .find(|node| node.id() == id)
        .unwrap()
        .origin()
}
fn extend_to_end(p: &mut Presentation) {
    let len = p
        .text
        .items()
        .iter()
        .find(|item| item.node_id() == "text")
        .unwrap()
        .layout()
        .text_len();
    for _ in 0..len {
        assert!(p.step_text_selection("text", true, true).unwrap());
        p.reveal_text_selection().unwrap();
    }
}
fn visible_focus(p: &Presentation) -> bool {
    let region = p
        .selection
        .as_ref()
        .unwrap()
        .focus_region(&p.text)
        .unwrap()
        .unwrap();
    let clip = p
        .text
        .items()
        .iter()
        .find(|item| item.node_id() == "text")
        .unwrap()
        .clip();
    !region.intersect(clip).empty()
}
#[test]
fn text_selection_reveal_vertical_endpoint_and_reverse_without_remeasurement() {
    let mut p = owner(
        r#"{"id":"root","type":"scroll","layout":{"height":60},"children":[{"id":"text","type":"text","props":{"text":"A\nB\nC\nD\nE\nF","selectable":true},"layout":{"height":300,"minHeight":300}}]}"#,
    );
    p.begin_text_selection([0.1, 8.]).unwrap();
    p.finish_text_selection([0.1, 8.]).unwrap();
    let measured = crate::surface_text::measure_calls();
    extend_to_end(&mut p);
    assert!(origin(&p, "text")[1] < 0.);
    assert!(visible_focus(&p));
    assert_eq!(
        p.selected_text().unwrap().as_deref(),
        Some("A\nB\nC\nD\nE\nF")
    );
    p.step_text_selection("text", false, false).unwrap();
    assert!(p.reveal_text_selection().unwrap());
    assert_eq!(origin(&p, "text")[1], 0.);
    assert_eq!(crate::surface_text::measure_calls(), measured);
}
#[test]
fn text_selection_reveal_horizontal_long_text_and_hidden_axis() {
    let mut p = owner(
        r#"{"id":"root","type":"view","layout":{"width":80,"height":60,"overflowX":"hidden"},"children":[{"id":"text","type":"text","props":{"text":"ABCDEFGHIJKLMNOPQRSTUVWXYZ","selectable":true},"layout":{"width":500,"minWidth":500,"height":40}}]}"#,
    );
    p.begin_text_selection([0.1, 8.]).unwrap();
    p.finish_text_selection([0.1, 8.]).unwrap();
    extend_to_end(&mut p);
    assert!(origin(&p, "text")[0] < 0.);
    assert!(visible_focus(&p));
    // hidden 可由键盘显露，不因此获得滚轮资格。
    assert!(!p.scroll.wheel(&p.geometry, "text", 0, -20., false));
    p.step_text_selection("text", false, false).unwrap();
    assert!(p.reveal_text_selection().unwrap());
    assert_eq!(origin(&p, "text")[0], 0.);
}
#[test]
fn text_selection_reveal_nested_scrolls_keep_endpoint_inside_both_clips() {
    let mut p = owner(
        r#"{"id":"root","type":"scroll","layout":{"height":80},"children":[{"id":"gap","type":"view","layout":{"height":60,"minHeight":60}},{"id":"inner","type":"scroll","layout":{"height":50,"minHeight":50},"children":[{"id":"text","type":"text","props":{"text":"A\nB\nC\nD\nE\nF","selectable":true},"layout":{"height":300,"minHeight":300}}]}]}"#,
    );
    assert!(p.begin_text_selection([0.1, 68.]).unwrap());
    p.finish_text_selection([0.1, 68.]).unwrap();
    extend_to_end(&mut p);
    assert!(origin(&p, "inner")[1] < 60.);
    assert!(origin(&p, "text")[1] < origin(&p, "inner")[1]);
    assert!(visible_focus(&p));
}
#[test]
fn text_selection_reveal_unreachable_clip_does_not_commit_outer_scroll() {
    let mut p = owner(
        r#"{"id":"root","type":"scroll","layout":{"height":80},"children":[{"id":"gap","type":"view","layout":{"height":120,"minHeight":120}},{"id":"zero","type":"scroll","layout":{"height":0,"minHeight":0},"children":[{"id":"text","type":"text","props":{"text":"A\nB","selectable":true},"layout":{"height":100,"minHeight":100}}]}]}"#,
    );
    // 模拟原先建立的选区遇到祖先变为零高；不能用这个内部设置证明鼠标能命中它。
    p.selection = Some(Selection {
        node: "text".into(),
        focus_node: "text".into(),
        anchor: 0,
        focus: 3,
        dragging: false,
        vertical_point: None,
        word: None,
    });
    let before = origin(&p, "zero");
    assert!(!p.reveal_text_selection().unwrap());
    assert_eq!(origin(&p, "zero"), before);
    p.selection.as_mut().unwrap().dragging = true;
    assert!(!p.reveal_text_selection().unwrap());
}

#[test]
fn text_selection_reveal_nonzero_text_clip_rejects_unreachable_endpoint() {
    let mut p = owner(
        r#"{"id":"root","type":"scroll","layout":{"height":80},"children":[{"id":"gap","type":"view","layout":{"height":120,"minHeight":120}},{"id":"text","type":"text","props":{"text":"A\nB\nC\nD\nE\nF","selectable":true},"layout":{"height":20,"minHeight":20}}]}"#,
    );
    p.selection = Some(Selection {
        node: "text".into(),
        focus_node: "text".into(),
        anchor: 0,
        focus: 11,
        dragging: false,
        vertical_point: None,
        word: None,
    });
    let before = origin(&p, "text");
    assert!(!p.reveal_text_selection().unwrap());
    assert_eq!(origin(&p, "text"), before);
    assert_eq!(
        p.selected_text().unwrap().as_deref(),
        Some("A\nB\nC\nD\nE\nF")
    );
}
