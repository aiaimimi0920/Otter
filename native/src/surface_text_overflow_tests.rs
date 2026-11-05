//! 真实 DirectWrite 最终帧的省略、固有尺寸、复制、重排与资源共享回归。
use super::*;
use crate::{surface_presentation::Presentation, surface_scene::Scene};
#[test]
fn text_overflow_trimmed_inline_hit_has_real_line_height_and_strict_pointer_bounds() {
    let prepared = frame(scene(
        "ellipsis",
        84.,
        32.,
        "长标题 e\u{301} 😀 Native text that must not be truncated",
    ));
    let layout = prepared.text().items()[0].layout();
    let sign = layout.cluster_rect(layout.text_len() - 1).unwrap();
    let point = [(sign.left + sign.right) / 2., (sign.top + sign.bottom) / 2.];
    let mut hit = DWRITE_HIT_TEST_METRICS::default();
    let (mut trailing, mut inside) = (
        windows::core::BOOL::default(),
        windows::core::BOOL::default(),
    );
    unsafe {
        layout
            .native
            .HitTestPoint(point[0], point[1], &mut trailing, &mut inside, &mut hit)
            .unwrap();
    }
    assert!(hit.isTrimmed.as_bool());
    layout.normalize_trimmed_hit(&mut hit).unwrap();
    assert_eq!(hit.top, sign.top);
    assert_eq!(hit.height, sign.bottom - sign.top);
    assert!(layout.hit_text(point).unwrap().inside);
    assert!(!layout.hit_text([sign.right + 4., point[1]]).unwrap().inside);
    let caret = layout.point_caret(point).unwrap();
    assert!(caret[1] >= sign.top && caret[1] <= sign.bottom);
}
fn style() -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 24.,
        weight: 400,
        whitespace: Whitespace::Normal,
        align: Align::Start,
        line_height: None,
    }
}
fn scene(overflow: &str, width: f32, height: f32, text: &str) -> Scene {
    Scene::parse(
        &serde_json::to_vec(&serde_json::json!({
        "id":"root","type":"column","layout":{"gap":0},"children":[{
        "id":"text","type":"text","props":{"text":text,"selectable":true},
        "layout":{"width":width,"height":height,"overflowX":"hidden","overflowY":"hidden"},
        "style":{"whiteSpace":"nowrap","textOverflow":overflow}}]}))
        .unwrap(),
    )
    .unwrap()
}
fn frame(scene: Scene) -> Presentation {
    Presentation::new(scene, style(), context(), [400., 80.]).unwrap()
}
fn context() -> crate::surface_length::Context {
    crate::surface_length::Context {
        axis: 400.,
        viewport: [400., 80.],
        container: [400., 80.],
        font: 24.,
        root_font: 24.,
        fraction: None,
    }
}
fn trimmed(layout: &Layout) -> bool {
    let mut lines = vec![DWRITE_LINE_METRICS::default(); layout.line_count() as usize];
    let mut count = 0;
    unsafe {
        layout
            .native
            .GetLineMetrics(Some(&mut lines), &mut count)
            .unwrap()
    };
    assert_eq!(count, 1);
    lines[0].isTrimmed.as_bool()
}
#[test]
fn text_overflow_final_frame_trims_without_changing_intrinsic_geometry_or_full_text() {
    let text = "长标题 e\u{301} 😀 Native text that must not be truncated";
    let clipped = frame(scene("clip", 84., 32., text));
    let ellipsis = frame(scene("ellipsis", 84., 32., text));
    let a = &clipped.text().items()[0];
    let b = &ellipsis.text().items()[0];
    assert!(!trimmed(a.layout()));
    assert!(
        trimmed(b.layout()),
        "missing native ellipsis, not just a saved CSS string"
    );
    assert_eq!(a.allocated(), b.allocated());
    assert_eq!(a.origin(), b.origin());
    assert_eq!(a.overflow(), b.overflow());
    assert_eq!(a.layout().size(), b.layout().size());
    assert!(b.overflow()[0] > 100.);
    assert_eq!(
        b.layout().selected_text(0..b.layout().text_len()).unwrap(),
        text
    );
    let mut position = 0;
    while position < b.layout().text_len() {
        let next = b.layout().step_text(position, true).unwrap();
        assert!(next > position);
        b.layout().selected_text(position..next).unwrap();
        position = next;
    }
    let sign = b.layout().cluster_rect(b.layout().text_len() - 1).unwrap();
    let hit = b
        .selection_start([(sign.left + sign.right) / 2., (sign.top + sign.bottom) / 2.])
        .unwrap()
        .unwrap();
    assert!(hit <= b.layout().text_len());
    assert!(b.selection_start([85., 12.]).unwrap().is_none());
    for rect in b.selection_rects(0..b.layout().text_len()).unwrap() {
        assert!(rect.left >= 0. && rect.right <= 84.);
    }
}
#[test]
fn text_overflow_hidden_suffix_navigation_selects_full_text_without_scroll_or_cluster_splitting() {
    let text = "ABC e\u{301} 👩\u{200d}💻 שלום hidden suffix";
    let mut prepared = frame(scene("ellipsis", 84., 32., text));
    assert!(prepared.begin_text_selection([1., 12.]).unwrap());
    prepared.finish_text_selection([1., 12.]).unwrap();
    assert!(prepared.line_edge_text_selection("text", true).unwrap());
    assert_eq!(prepared.selected_text().unwrap().as_deref(), Some(text));
    assert!(!prepared.reveal_text_selection().unwrap());
    let layout = prepared.text().items()[0].layout();
    let end = layout.text_len();
    let hidden = layout.step_text(end, false).unwrap();
    assert!(hidden < end);
    assert_eq!(layout.step_text(hidden, true).unwrap(), end);
    let suffix = layout.selected_text(hidden..end).unwrap();
    assert!(suffix.ends_with("hidden suffix"));
    // 被省略后缀作为一个视觉 cluster，允许一次跳过；仍保留完整逻辑内容供复制。
    let sign = layout.cluster_rect(end - 1).unwrap();
    let y = (sign.top + sign.bottom) / 2.;
    let quarter = (sign.right - sign.left) / 4.;
    assert_eq!(
        layout.hit_text([sign.left + quarter, y]).unwrap().position,
        hidden
    );
    assert_eq!(
        layout.hit_text([sign.right - quarter, y]).unwrap().position,
        end
    );
    for x in [sign.left + quarter, sign.right - quarter] {
        assert!(prepared.begin_word_selection([x, y]).unwrap());
        assert_eq!(
            prepared.selected_text().unwrap().as_deref(),
            Some(suffix.as_str())
        );
        prepared.finish_text_selection([x, y]).unwrap();
        assert_eq!(
            prepared.selected_text().unwrap().as_deref(),
            Some(suffix.as_str())
        );
    }
}
#[test]
fn text_overflow_scroll_cache_keeps_trimming_and_never_remeasures_or_reveals_hidden_suffix() {
    let source=Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":60},"children":[{"id":"text","type":"text","props":{"text":"MMMMMMMMMMMMMMMM hidden suffix","selectable":true},"layout":{"width":84,"height":32,"minHeight":32,"overflowX":"hidden","overflowY":"hidden"},"style":{"whiteSpace":"nowrap","textOverflow":"ellipsis"}},{"id":"tail","type":"view","layout":{"height":200,"minHeight":200}}]}"#).unwrap();
    let mut prepared = frame(source);
    let original = prepared.text().items()[0].origin();
    let calls = measure_calls();
    assert!(prepared.begin_text_selection([1., 12.]).unwrap());
    prepared.finish_text_selection([1., 12.]).unwrap();
    assert!(prepared.line_edge_text_selection("text", true).unwrap());
    assert!(!prepared.reveal_text_selection().unwrap());
    assert_eq!(prepared.text().items()[0].origin(), original);
    assert!(prepared.pan("root", [0., 4.]).unwrap());
    assert_ne!(prepared.text().items()[0].origin(), original);
    assert!(trimmed(prepared.text().items()[0].layout()));
    assert!(prepared
        .selected_text()
        .unwrap()
        .unwrap()
        .ends_with("hidden suffix"));
    assert!(prepared.pan("root", [0., -4.]).unwrap());
    assert_eq!(prepared.text().items()[0].origin(), original);
    assert_eq!(measure_calls(), calls);
}
#[test]
fn text_overflow_rejects_actual_multiline_and_invalid_native_values() {
    for separator in ['\u{2028}', '\u{2029}'] {
        let source = scene("ellipsis", 84., 32., &format!("A{separator}B"));
        assert!(Presentation::new(source, style(), context(), [400., 80.]).is_err());
    }
    let source = Scene::parse(
        br#"{"id":"t","type":"text","props":{"text":"text"},"style":{"textOverflow":false}}"#,
    )
    .unwrap();
    assert!(Presentation::new(source, style(), context(), [400., 80.]).is_err());
}
#[path = "surface_text_overflow_pixels_tests.rs"]
mod pixels;
#[test]
fn text_overflow_resize_never_mutates_clones_or_ellipsizes_vertical_clipping() {
    let text = "Native overflow";
    let mut prepared = frame(scene("ellipsis", 60., 32., text));
    let old = prepared.text().items()[0].layout().clone();
    assert!(trimmed(&old));
    prepared
        .replace(
            scene("ellipsis", 380., 2., text),
            style(),
            context(),
            [400., 80.],
        )
        .unwrap();
    let next = &prepared.text().items()[0];
    assert!(!trimmed(next.layout()), "height clipping is not line-clamp");
    assert!(
        trimmed(&old),
        "a shared COM layout must not be mutated by a later frame"
    );
    assert!(next.overflow()[1] > 0.);
    for width in [0., 1., 4., 20., 400.] {
        let prepared = frame(scene("ellipsis", width, 32., "中😀ABC"));
        if width == 0. {
            assert!(prepared.text().items().is_empty());
        } else {
            let item = &prepared.text().items()[0];
            assert_eq!(
                item.layout()
                    .selected_text(0..item.layout().text_len())
                    .unwrap(),
                "中😀ABC"
            );
            assert_eq!(trimmed(item.layout()), width < 100.);
        }
    }
}
