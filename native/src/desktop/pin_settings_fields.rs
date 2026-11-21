//! 当前可用的贴图设置字段；原主干 SVG 不重画，图像自然比例来自实际显示源。
use super::*;
fn icon(name: &str) -> neuro_mini_ui::Icon {
    neuro_mini_ui::Icon::svg(match name {
        "FlipX" => include_str!("../../assets/edit-strip/SettingsFlipX.svg"),
        "FlipY" => include_str!("../../assets/edit-strip/SettingsFlipY.svg"),
        "Fit" => include_str!("../../assets/edit-strip/SettingsFit.svg"),
        "Width" => include_str!("../../assets/edit-strip/SettingsWidth.svg"),
        "Height" => include_str!("../../assets/edit-strip/SettingsHeight.svg"),
        "Border" => include_str!("../../assets/edit-strip/SettingsBorder.svg"),
        "Radius" => include_str!("../../assets/edit-strip/SettingsRadius.svg"),
        _ => include_str!("../../assets/edit-strip/SettingsOpacity.svg"),
    })
}
pub(super) fn fields(
    frame: PinRect,
    dpi: u32,
    opacity: f32,
    style: crate::pin_frame_style::Style,
    mirror: crate::editor::MirrorState,
) -> Vec<Field> {
    let logical = |v: u32| (f64::from(v) * 96.0 / f64::from(dpi)).round() as i32;
    let action = |id, label, name| Field {
        id,
        label,
        icon: Some(icon(name)),
        kind: Kind::Action {
            value: String::new(),
            active: match id {
                FLIP_X => mirror.x,
                FLIP_Y => mirror.y,
                BORDER => style.border().is_some(),
                _ => false,
            },
            width: 28,
        },
    };
    vec![
        action(FLIP_X, "水平镜像", "FlipX"),
        action(FLIP_Y, "垂直镜像", "FlipY"),
        action(BORDER, "外边框", "Border"),
        action(FIT, "合拢外框", "Fit"),
        Field {
            id: WIDTH,
            label: "宽度",
            icon: Some(icon("Width")),
            kind: Kind::Number {
                value: logical(frame.width).clamp(32, 8192),
                min: 32,
                max: 8192,
                suffix: "",
            },
        },
        Field {
            id: HEIGHT,
            label: "等比高度",
            icon: Some(icon("Height")),
            kind: Kind::ReadOnly {
                value: logical(frame.height).to_string(),
            },
        },
        Field {
            id: OPACITY,
            label: "不透明度（%）",
            icon: Some(icon("Opacity")),
            kind: Kind::Number {
                value: (opacity * 100.0).round() as i32,
                min: 0,
                max: 100,
                suffix: "%",
            },
        },
        Field {
            id: RADIUS,
            label: "圆角",
            icon: Some(icon("Radius")),
            kind: Kind::Number {
                value: style.radius().round() as i32,
                min: 0,
                max: 128,
                suffix: "",
            },
        },
    ]
}
pub(super) fn image_size(state: &State, c: &super::super::canvas::Canvas) -> Option<(u32, u32)> {
    let display = state.resolved_display.borrow();
    let image = display
        .as_ref()
        .map(|f| &f.image)
        .or_else(|| {
            c.minified_source
                .as_ref()
                .filter(|s| s.matches(&c.editor))
                .map(|s| s.image())
        })
        .or(c.image.as_ref())?;
    Some((image.width, image.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opacity_uses_standalone_percent_while_width_and_radius_remain_plain_numbers() {
        let values = fields(
            PinRect {
                x: 0,
                y: 0,
                width: 100,
                height: 80,
            },
            96,
            0.5,
            Default::default(),
            Default::default(),
        );
        for (id, unit) in [(WIDTH, ""), (OPACITY, "%"), (RADIUS, "")] {
            let value = values.iter().find(|f| f.id == id).unwrap();
            assert!(matches!(value.kind, Kind::Number { suffix, .. } if suffix == unit));
        }
    }
    #[test]
    fn mirror_pressed_is_per_axis_not_a_shared_label_state() {
        for (x, y) in [(false, false), (true, false), (false, true), (true, true)] {
            let values = fields(
                PinRect {
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 20,
                },
                96,
                1.0,
                Default::default(),
                crate::editor::MirrorState { x, y },
            );
            for (id, expected) in [(FLIP_X, x), (FLIP_Y, y), (BORDER, false), (FIT, false)] {
                let field = values.iter().find(|f| f.id == id).unwrap();
                assert!(matches!(field.kind, Kind::Action { active, .. } if active == expected));
            }
        }
    }
}
