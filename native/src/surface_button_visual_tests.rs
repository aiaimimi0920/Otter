//! 状态色的真实 WIC 像素合同，覆盖禁用和显式样式优先级；共用现有资源还原断言。
use super::*;
use crate::surface_button_visual::State;

#[test]
fn surface_button_hover_and_pressed_colors_reach_pixels() {
    let json = br#"{"id":"button","type":"button","props":{"label":"X"},"layout":{"width":64,"height":64}}"#;
    for (state, expected) in [
        (State::default(), [32_u8, 23, 17, 255]),
        (
            State {
                hover: Some("button"),
                ..Default::default()
            },
            [56, 255, 217, 255],
        ),
        (
            State {
                pressed: Some("button"),
                ..Default::default()
            },
            [46, 209, 178, 255],
        ),
    ] {
        let pixels = render_visual(json, true, state);
        let sample = &pixels[(12 * WIDTH + 12) * 4..(12 * WIDTH + 13) * 4];
        assert!(sample.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1));
        if state.hover.is_some() || state.pressed.is_some() {
            assert!((20..44).any(|y| (20..44).any(|x| {
                let p = &pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4];
                p[0] < 40 && p[1] < 40 && p[2] < 40 && p[3] == 255
            })));
        }
    }
}
#[test]
fn surface_button_disabled_and_explicit_styles_ignore_dynamic_colors() {
    for json in [
        br#"{"id":"button","type":"button","props":{"disabled":true,"label":"X"}}"#.as_slice(),
        br#"{"id":"button","type":"button","props":{"label":"X"},"style":{"background":"blue","borderColor":"green","color":"red"}}"#.as_slice(),
    ] {
        let normal = render_visual(json, true, State::default());
        for state in [State { hover: Some("button"), ..Default::default() }, State { pressed: Some("button"), ..Default::default() }] {
            assert_eq!(render_visual(json, true, state), normal);
        }
    }
}
