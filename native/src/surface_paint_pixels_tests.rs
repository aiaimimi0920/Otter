//! 真实 WIC/Direct2D 像素验证颜色、alpha、继承、裁剪及父子/兄弟绘制顺序。
use super::*;
#[path = "surface_button_visual_tests.rs"]
mod button_visual;
#[path = "surface_pixels_fixture.rs"]
mod fixture;
#[path = "surface_scroll_pixels_tests.rs"]
mod scroll_pixels;
pub(super) use fixture::render as render_switches;
pub(super) const WIDTH: usize = 64;
#[test]
fn surface_button_focus_respects_sibling_occlusion_and_group_opacity() {
    let pixels = render_focus(br#"{"id":"root","type":"stack","children":[{"id":"button","type":"button","layout":{"width":64,"height":64}},{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#, true, Some("button"));
    assert!(pixels.chunks_exact(4).all(|p| p == [255, 0, 0, 255]));
    let pixels = render_focus(br#"{"id":"root","type":"view","style":{"opacity":0.5},"children":[{"id":"button","type":"button","layout":{"width":64,"height":64}}]}"#, true, Some("button"));
    assert!((127..=128).contains(&pixels[32 * 4 + 3]));
}
#[test]
fn surface_button_keyboard_focus_ring_draws_and_disabled_suppresses_it() {
    for disabled in [false, true] {
        let json = format!(
            r#"{{"id":"button","type":"button","props":{{"disabled":{disabled}}},"layout":{{"width":64,"height":64}}}}"#
        );
        let pixels = render_focus(json.as_bytes(), true, Some("button"));
        let pixel = &pixels[32 * 4..33 * 4];
        if disabled {
            assert_ne!(pixel, [56, 255, 217, 255]);
        } else {
            assert_eq!(pixel, [56, 255, 217, 255]);
        }
    }
}
#[test]
fn surface_button_paints_background_border_and_label() {
    let empty = render(br#"{"id":"button","type":"button","layout":{"width":64,"height":64}}"#);
    let labeled = render(br#"{"id":"button","type":"button","props":{"label":"X"},"layout":{"width":64,"height":64}}"#);
    let pixel = |pixels: &[u8], x: usize, y: usize| {
        pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4].to_vec()
    };
    assert_eq!(pixel(&empty, 32, 32), [32, 23, 17, 255]);
    assert_eq!(pixel(&empty, 0, 0), [0, 0, 0, 0]);
    assert_ne!(pixel(&empty, 32, 0), pixel(&empty, 32, 32));
    assert!(labeled
        .chunks_exact(4)
        .zip(empty.chunks_exact(4))
        .any(|(a, b)| a[0] > b[0] + 20 && a[1] > b[1] + 20));
}
#[test]
fn hidden_scene_and_hidden_stack_overlay_produce_no_pixels() {
    let empty = render(
        br#"{"id":"root","type":"view","props":{"visible":false},"style":{"background":"red"}}"#,
    );
    assert!(empty.iter().all(|value| *value == 0));
    let visible = render(br#"{"id":"root","type":"stack","style":{"background":"blue"},"children":[{"id":"hidden","type":"view","props":{"visible":false},"style":{"background":"red"}}]}"#);
    assert!(visible
        .chunks_exact(4)
        .all(|pixel| pixel == [255, 0, 0, 255]));
}
#[test]
fn progress_pixels_show_clamped_fraction_and_restore_draw_state() {
    for (value, left, right) in [(0., false, false), (0.5, true, false), (2., true, true)] {
        let json =
            format!(r#"{{"id":"root","type":"progress","props":{{"value":{value},"max":1}}}}"#);
        let pixels = render(json.as_bytes());
        for (x, active) in [(16, left), (48, right)] {
            assert_eq!(
                &pixels[(8 * WIDTH + x) * 4..(8 * WIDTH + x + 1) * 4],
                if active {
                    &[56, 255, 217, 255]
                } else {
                    &[32, 23, 17, 255]
                }
            );
        }
        assert_eq!(
            &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
            &[0, 0, 0, 0]
        );
    }
}
#[test]
fn divider_paints_only_one_dark_theme_pixel_row_even_with_ignored_style() {
    let pixels =
        render(br#"{"id":"root","type":"divider","style":{"opacity":0,"background":"red"}}"#);
    assert!(pixels[..WIDTH * 4]
        .chunks_exact(4)
        .all(|pixel| pixel == [51, 43, 36, 255]));
    assert!(pixels[WIDTH * 4..]
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 0, 0, 0]));
}
#[test]
fn spacer_style_paints_allocated_flex_space_without_text_resources() {
    let pixels = render(br#"{"id":"root","type":"row","children":[{"id":"space","type":"spacer","style":{"background":"blue"}}]}"#);
    assert!(pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [255, 0, 0, 255]));
}
#[test]
fn absolute_auto_root_paints_only_between_vertical_anchors() {
    let pixels = render_scene(br#"{"id":"root","type":"view","layout":{"position":"absolute","left":8,"width":16,"top":8,"bottom":8,"height":"auto"},"style":{"background":"blue"}}"#, false);
    assert_eq!(
        &pixels[(4 * WIDTH + 16) * 4..(4 * WIDTH + 17) * 4],
        &[0, 0, 0, 0]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 16) * 4..(32 * WIDTH + 17) * 4],
        &[255, 0, 0, 255]
    );
    assert_eq!(
        &pixels[(60 * WIDTH + 16) * 4..(60 * WIDTH + 17) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn natural_root_background_ends_at_content_height() {
    let pixels = render_scene(br#"{"id":"root","type":"view","style":{"background":"blue"},"children":[{"id":"child","type":"view","layout":{"width":8,"height":16}}]}"#, false);
    assert_eq!(
        &pixels[(8 * WIDTH + 32) * 4..(8 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn positioned_root_moves_pixels_but_not_host_clip() {
    let pixels = render(br#"{"id":"root","type":"view","layout":{"position":"absolute","right":0,"bottom":0,"width":16,"height":16},"style":{"background":"blue"}}"#);
    assert_eq!(
        &pixels[(8 * WIDTH + 8) * 4..(8 * WIDTH + 9) * 4],
        &[0, 0, 0, 0]
    );
    assert_eq!(
        &pixels[(56 * WIDTH + 56) * 4..(56 * WIDTH + 57) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn explicit_root_background_does_not_fill_remaining_host_space() {
    let pixels = render(br#"{"id":"root","type":"view","layout":{"width":32,"height":16},"style":{"background":"blue"}}"#);
    assert_eq!(
        &pixels[(8 * WIDTH + 8) * 4..(8 * WIDTH + 9) * 4],
        &[255, 0, 0, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn single_axis_overflow_also_clips_other_axis_and_rounds_children() {
    for key in ["overflowX", "overflowY"] {
        let json = format!(
            r#"{{"id":"root","type":"view","children":[{{"id":"clip","type":"view","layout":{{"width":32,"height":32,"{key}":"hidden"}},"style":{{"borderRadius":16}},"children":[{{"id":"fill","type":"view","layout":{{"width":64,"height":64}},"style":{{"background":"blue"}}}}]}}]}}"#
        );
        let pixels = render(json.as_bytes());
        for (x, y) in [(1, 1), (40, 16), (16, 40)] {
            assert_eq!(
                &pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4],
                &[0, 0, 0, 0]
            );
        }
        assert_eq!(
            &pixels[(16 * WIDTH + 16) * 4..(16 * WIDTH + 17) * 4],
            &[255, 0, 0, 255]
        );
    }
}
#[test]
fn rectangular_overflow_clips_child_before_parent_border() {
    let pixels = render(br#"{"id":"root","type":"view","layout":{"position":"relative","overflowX":"hidden","overflowY":"hidden"},"style":{"borderWidth":8,"borderColor":"red"},"children":[{"id":"fill","type":"view","layout":{"position":"absolute","left":0,"top":0,"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert_eq!(
        &pixels[(32 * WIDTH + 60) * 4..(32 * WIDTH + 61) * 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn rounded_overflow_uses_inner_radius_without_erasing_border_corner() {
    let pixels = render(br#"{"id":"root","type":"view","layout":{"position":"relative","overflowX":"hidden","overflowY":"hidden"},"style":{"borderWidth":8,"borderRadius":24,"borderColor":"red"},"children":[{"id":"fill","type":"view","layout":{"position":"absolute","left":0,"top":0,"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert_eq!(
        &pixels[(10 * WIDTH + 10) * 4..(10 * WIDTH + 11) * 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn border_ring_keeps_transparent_center_and_inherits_current_color() {
    let pixels = render(br#"{"id":"root","type":"view","style":{"borderWidth":8,"color":"red"}}"#);
    assert_eq!(
        &pixels[(32 * WIDTH + 2) * 4..(32 * WIDTH + 3) * 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn rounded_border_preserves_outer_corners_and_background_in_inner_hole() {
    let pixels = render(br#"{"id":"root","type":"view","style":{"borderWidth":8,"borderRadius":24,"borderColor":"red","background":"blue"}}"#);
    assert_eq!(&pixels[(WIDTH + 1) * 4..(WIDTH + 2) * 4], &[0, 0, 0, 0]);
    assert_eq!(
        &pixels[(32 * WIDTH + 2) * 4..(32 * WIDTH + 3) * 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn thick_border_fills_collapsed_inner_box_and_alpha_is_not_doubled_at_corners() {
    let pixels = render(br#"{"id":"root","type":"view","style":{"borderWidth":40,"borderColor":"rgba(255,0,0,0.5)"}}"#);
    for (x, y) in [(2, 2), (32, 32), (61, 61)] {
        let pixel = &pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4];
        assert_eq!(&pixel[..2], &[0, 0]);
        assert!((127..=128).contains(&pixel[2]));
        assert!((127..=128).contains(&pixel[3]));
    }
}
#[test]
fn double_axis_overflow_clips_child_pixels_to_parent_rounding() {
    let pixels = render(br#"{"id":"root","type":"view","layout":{"overflowX":"hidden","overflowY":"hidden"},"style":{"borderRadius":"50%"},"children":[{"id":"fill","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert_eq!(&pixels[(WIDTH + 1) * 4..(WIDTH + 2) * 4], &[0, 0, 0, 0]);
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn rounded_clip_composes_with_group_opacity_and_ends_before_next_sibling() {
    let pixels = render(br#"{"id":"root","type":"stack","children":[{"id":"round","type":"view","layout":{"width":64,"height":64,"overflowX":"hidden","overflowY":"hidden"},"style":{"opacity":0.5,"borderRadius":32},"children":[{"id":"fill","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]},{"id":"sibling","type":"view","layout":{"width":16,"height":16},"style":{"background":"red"}}]}"#);
    assert_eq!(&pixels[(WIDTH + 1) * 4..(WIDTH + 2) * 4], &[0, 0, 255, 255]);
    let middle = &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4];
    assert!((127..=128).contains(&middle[0]));
    assert!((127..=128).contains(&middle[3]));
    assert_eq!(
        &pixels[(62 * WIDTH + 62) * 4..(62 * WIDTH + 63) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn background_container_units_keep_host_query_context_in_nested_layout() {
    // context() 的查询容器宽 400；4cqw=16，不能改用直接父节点宽 32 得到 1.28。
    // (2, 2) 明确位于半径 16 的弧外，避开 (4, 4) 处的抗锯齿覆盖区域。
    let pixels = render(br#"{"id":"root","type":"column","children":[{"id":"parent","type":"column","layout":{"width":32,"height":64},"children":[{"id":"rounded","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue","borderRadius":"4cqw"}}]}]}"#);
    assert_eq!(
        &pixels[(2 * WIDTH + 2) * 4..(2 * WIDTH + 3) * 4],
        &[0, 0, 0, 0]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn rounded_background_leaves_corners_transparent_and_center_opaque() {
    let pixels = render(
        br#"{"id":"root","type":"view","style":{"background":"blue","borderRadius":"50%"}}"#,
    );
    assert_eq!(&pixels[(WIDTH + 1) * 4..(WIDTH + 2) * 4], &[0, 0, 0, 0]);
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn ancestor_clip_does_not_recalculate_rounded_background_geometry() {
    let pixels = render(br#"{"id":"root","type":"column","children":[{"id":"clip","type":"column","layout":{"width":32,"height":64,"overflowX":"hidden"},"children":[{"id":"circle","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue","borderRadius":"50%"}}]}]}"#);
    assert_eq!(
        &pixels[(4 * WIDTH + 30) * 4..(4 * WIDTH + 31) * 4],
        &[255, 0, 0, 255]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 40) * 4..(32 * WIDTH + 41) * 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn parent_background_radius_does_not_clip_ordinary_child_content() {
    let pixels = render(br#"{"id":"root","type":"stack","style":{"background":"blue","borderRadius":32},"children":[{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"red"}}]}"#);
    assert_eq!(&pixels[(WIDTH + 1) * 4..(WIDTH + 2) * 4], &[0, 0, 255, 255]);
}
#[test]
fn rounded_background_em_uses_inherited_computed_font() {
    let pixels = render(br#"{"id":"root","type":"column","style":{"fontSize":32},"children":[{"id":"circle","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue","borderRadius":"1em"}}]}"#);
    assert_eq!(
        &pixels[(8 * WIDTH + 8) * 4..(8 * WIDTH + 9) * 4],
        &[0, 0, 0, 0]
    );
    assert_eq!(
        &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn opacity_is_numeric_clamped_and_not_inherited_as_a_style_value() {
    for (value, visible) in [("2", true), ("-1", false), ("\"0\"", true)] {
        let scene = format!(
            r#"{{"id":"root","type":"view","style":{{"background":"blue","opacity":{value}}}}}"#
        );
        let pixels = render(scene.as_bytes());
        assert!(pixels.chunks_exact(4).all(|pixel| pixel
            == if visible {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 0, 0]
            }));
    }
}
#[test]
fn opacity_applies_once_to_overlapping_backgrounds_as_a_group() {
    let pixels = render(br#"{"id":"root","type":"stack","style":{"opacity":0.5,"background":"red"},"children":[{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    let pixel = &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4];
    assert!((127..=128).contains(&pixel[0]));
    assert_eq!(&pixel[1..3], &[0, 0]);
    assert!((127..=128).contains(&pixel[3]));
}
#[test]
fn nested_opacity_composes_and_zero_opacity_subtree_draws_nothing() {
    let pixels = render(br#"{"id":"root","type":"stack","style":{"opacity":0.5},"children":[{"id":"visible","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue","opacity":0.5}},{"id":"hidden","type":"view","style":{"opacity":0,"background":"red"},"children":[{"id":"opaque-child","type":"view","layout":{"width":64,"height":64},"style":{"background":"red","opacity":1}}]}]}"#);
    let pixel = &pixels[(32 * WIDTH + 32) * 4..(32 * WIDTH + 33) * 4];
    assert!((63..=65).contains(&pixel[0]));
    assert_eq!(&pixel[1..3], &[0, 0]);
    assert!((63..=65).contains(&pixel[3]));
}
#[test]
fn opacity_layer_preserves_visible_child_overflow() {
    let pixels = render(br#"{"id":"root","type":"column","children":[{"id":"group","type":"column","layout":{"width":16,"height":32},"style":{"opacity":0.5},"children":[{"id":"wide","type":"view","layout":{"width":64,"height":32},"style":{"background":"blue"}}]}]}"#);
    let pixel = &pixels[(8 * WIDTH + 32) * 4..(8 * WIDTH + 33) * 4];
    assert!((127..=128).contains(&pixel[0]));
    assert!((127..=128).contains(&pixel[3]));
}
pub(super) fn render(json: &[u8]) -> Vec<u8> {
    render_scene(json, true)
}
fn render_scene(json: &[u8], fixed_height: bool) -> Vec<u8> {
    render_focus(json, fixed_height, None)
}
fn render_focus(json: &[u8], fixed_height: bool, focus: Option<&str>) -> Vec<u8> {
    render_visual(
        json,
        fixed_height,
        crate::surface_button_visual::State {
            focus,
            ..Default::default()
        },
    )
}
fn render_visual(
    json: &[u8],
    fixed_height: bool,
    visual: crate::surface_button_visual::State<'_>,
) -> Vec<u8> {
    render_prepared(json, fixed_height, visual, |_| {})
}
fn render_prepared(
    json: &[u8],
    fixed_height: bool,
    visual: crate::surface_button_visual::State<'_>,
    prepare: impl FnOnce(&mut crate::surface_presentation::Presentation),
) -> Vec<u8> {
    fixture::render(json, fixed_height, visual, prepare, |_| None)
}
#[test]
fn inherited_current_color_and_explicit_text_override_reach_pixels() {
    let pixels = render(br#"{"id":"root","type":"column","style":{"color":"red"},"children":[{"id":"a","type":"text","layout":{"width":64,"height":32},"props":{"text":"MMMM"},"style":{"color":"currentColor"}},{"id":"b","type":"text","layout":{"width":64,"height":32},"props":{"text":"MMMM"},"style":{"color":"blue"}}]}"#);
    assert!(pixels
        .chunks_exact(4)
        .take(WIDTH * 32)
        .any(|p| p[2] > 0 && p[1] == 0 && p[0] == 0));
    assert!(pixels
        .chunks_exact(4)
        .skip(WIDTH * 32)
        .any(|p| p[0] > 0 && p[1] == 0 && p[2] == 0));
}
#[test]
fn later_stack_background_covers_earlier_text_in_scene_order() {
    let pixels = render(br#"{"id":"root","type":"stack","style":{"background":"red"},"children":[{"id":"text","type":"text","props":{"text":"MMMM"},"style":{"fontSize":40,"color":"green"}},{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert!(pixels.chunks_exact(4).all(|p| p == [255, 0, 0, 255]));
}
#[test]
fn alpha_background_respects_ancestor_clip_without_changing_uncovered_pixels() {
    let pixels = render(br#"{"id":"root","type":"column","style":{"background":"blue"},"children":[{"id":"clip","type":"column","layout":{"width":16,"height":32,"overflowX":"hidden"},"children":[{"id":"fill","type":"view","layout":{"width":64,"height":32},"style":{"background":"rgba(255,0,0,0.5)"}}]}]}"#);
    let inside = &pixels[(8 * WIDTH + 8) * 4..(8 * WIDTH + 9) * 4];
    assert!((127..=128).contains(&inside[0]));
    assert_eq!(inside[1], 0);
    assert!((127..=128).contains(&inside[2]));
    assert_eq!(inside[3], 255);
    assert_eq!(
        &pixels[(8 * WIDTH + 24) * 4..(8 * WIDTH + 25) * 4],
        &[255, 0, 0, 255]
    );
}
