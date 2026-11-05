//! Switch 动态状态的真实 WIC 像素；复用统一夹具验证画刷与后续绘制不受污染。
use crate::surface_button_visual::State;
fn render(checked: bool, disabled: bool, value: Option<bool>, state: State<'_>) -> Vec<u8> {
    let json = serde_json::json!({"id":"switch","type":"switch","props":{"value":checked,"disabled":disabled}});
    super::paint_pixels::render_switches(
        json.to_string().as_bytes(),
        false,
        state,
        |_| {},
        |id| {
            assert_eq!(id, "switch");
            value
        },
    )
}
fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
}
#[test]
fn surface_switch_visual_local_override_changes_thumb_and_track_both_directions() {
    for source in [false, true] {
        assert_eq!(
            render(source, false, Some(!source), State::default()),
            render(!source, false, None, State::default())
        );
        assert_ne!(
            render(source, false, Some(!source), State::default()),
            render(source, false, None, State::default())
        );
    }
}
#[test]
fn surface_switch_visual_checked_hover_and_press_follow_canonical_derivatives() {
    for (state, expected) in [
        (
            State {
                hover: Some("switch"),
                ..Default::default()
            },
            [84_u8, 255, 222, 255],
        ),
        (
            State {
                pressed: Some("switch"),
                ..Default::default()
            },
            [46_u8, 209, 178, 255],
        ),
    ] {
        let pixels = render(true, false, None, state);
        assert!(pixel(&pixels, 24, 9)
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 1));
    }
}
#[test]
fn surface_switch_visual_unchecked_hover_changes_control_not_active_signal() {
    let pixels = render(
        false,
        false,
        None,
        State {
            hover: Some("switch"),
            ..Default::default()
        },
    );
    assert_eq!(pixel(&pixels, 39, 9), [44, 34, 24, 255]);
    assert_eq!(pixel(&pixels, 24, 9), [159, 154, 146, 255]);
}
#[test]
fn surface_switch_visual_disabled_suppresses_hover_press_focus_but_keeps_value_position() {
    for checked in [false, true] {
        let normal = render(checked, true, None, State::default());
        let attempted = render(
            checked,
            true,
            None,
            State {
                focus: Some("switch"),
                hover: Some("switch"),
                pressed: Some("switch"),
            },
        );
        assert_eq!(normal, attempted);
        let thumb_x = if checked { 39 } else { 24 };
        assert!(pixel(&normal, thumb_x, 9)
            .iter()
            .zip([108_u8, 102, 94, 255])
            .all(|(a, b)| a.abs_diff(b) <= 1));
        assert_ne!(normal, render(checked, false, None, State::default()));
    }
    assert_ne!(
        render(false, true, None, State::default()),
        render(true, true, None, State::default())
    );
}
#[test]
fn surface_switch_visual_keyboard_focus_is_visible_without_changing_local_value() {
    let pixels = render(
        false,
        false,
        Some(true),
        State {
            focus: Some("switch"),
            ..Default::default()
        },
    );
    assert_eq!(pixel(&pixels, 0, 9), [56, 255, 217, 255]);
    assert_eq!(pixel(&pixels, 24, 9), [56, 255, 217, 255]);
    assert!(pixel(&pixels, 39, 9)[1] < 40);
}
