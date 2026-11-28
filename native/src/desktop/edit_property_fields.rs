//! Field projections for baseline secondary bars; map business values to control presentation explicitly once.
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, StrokeStyle},
};
use neuro_mini_ui::{PropertyField as Field, PropertyKind as Kind};
pub(super) struct View {
    pub tool: Tool,
    pub selected: Option<usize>,
    pub style: StrokeStyle,
}
pub(super) fn view(c: &Canvas) -> Option<View> {
    if c.crop_mode || c.minified.is_some() {
        return None;
    }
    let selected = if c.tool == Tool::Select {
        Some(c.selected_index()?)
    } else if c.tool == Tool::Magnifier {
        c.selected_index().filter(|&i| {
            c.editor
                .elements()
                .get(i)
                .is_some_and(|s| s.tool == Tool::Magnifier)
        })
    } else {
        None
    };
    let (tool, style) = if let Some(index) = selected {
        let s = &c.editor.elements()[index];
        if !s.tool.spotlight()
            && !matches!(
                s.tool,
                Tool::Text
                    | Tool::Serial
                    | Tool::Line
                    | Tool::Arrow
                    | Tool::Bezier
                    | Tool::Magnifier
            )
        {
            return None;
        }
        (s.tool, s.style())
    } else {
        (c.tool, c.editor.style())
    };
    if matches!(tool, Tool::Pan | Tool::Select) {
        return None;
    }
    Some(View {
        tool,
        selected,
        style,
    })
}
fn field(id: u16, label: &'static str, kind: Kind) -> Field {
    Field {
        id,
        label: crate::ui_text::text(label),
        icon: icon(id),
        kind,
    }
}
fn icon(id: u16) -> Option<neuro_mini_ui::Icon> {
    use neuro_mini_ui::Icon;
    macro_rules! svg {
        ($name:literal) => {
            Some(Icon::svg(include_str!(concat!(
                "../../assets/edit-strip/",
                $name,
                ".svg"
            ))))
        };
    }
    match id {
        WIDTH => svg!("PropertyLineWidth"),
        RADIUS => svg!("PropertyRadius"),
        EFFECT_RECT | TEXT_BOX => svg!("PropertyConstrain"),
        SERIAL_RADIUS => svg!("PropertySerialRadius"),
        STEP => svg!("PropertyStep"),
        OPACITY => svg!("PropertyOpacity"),
        SIDES => svg!("PropertyPolygonSides"),
        ARROW => svg!("PropertyArrowHead"),
        ANGLE => svg!("PropertyAngleSnap"),
        GLOW => svg!("PropertyHighlighterGlow"),
        ONLY_MARKS => svg!("PropertyAnnotationsOnlyFocused"),
        SIZE => svg!("PropertyTextSize"),
        FONT => svg!("TextTool"),
        CELL => svg!("MosaicTool"),
        STRENGTH => svg!("BlurTool"),
        SMOOTH => svg!("PropertySmoothing"),
        CONSTRAIN => svg!("PropertyConstrain"),
        RATIO => svg!("PropertyAspectRatio"),
        SHAPE => svg!("RectTool"),
        _ => None,
    }
}

pub(super) fn shape_icon(index: usize) -> neuro_mini_ui::Icon {
    use neuro_mini_ui::Icon;
    match index {
        1 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2.5" y="4.5" width="19" height="15" rx="4.2" /></svg>"#,
        ),
        2 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><ellipse cx="12" cy="12" rx="9.5" ry="7.5" /></svg>"#,
        ),
        _ => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2.5" y="4.5" width="19" height="15" /></svg>"#,
        ),
    }
}

pub(super) fn shape_button_icon(index: usize) -> neuro_mini_ui::Icon {
    use neuro_mini_ui::Icon;
    // Matches primary Shape tool button: 28x28 square button with centered shape + bottom-right corner dropdown caret.
    match index {
        1 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><rect x="1.5" y="2.5" width="19" height="15" rx="4.5" /><path d="M20.5 22.5 25.5 27.5 30.5 22.5Z" fill="currentColor" stroke="none" /></svg>"#,
        ),
        2 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><ellipse cx="11" cy="10" rx="9.5" ry="7.5" /><path d="M20.5 22.5 25.5 27.5 30.5 22.5Z" fill="currentColor" stroke="none" /></svg>"#,
        ),
        _ => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><rect x="1.5" y="2.5" width="19" height="15" /><path d="M20.5 22.5 25.5 27.5 30.5 22.5Z" fill="currentColor" stroke="none" /></svg>"#,
        ),
    }
}

fn shape_dropdown(index: usize) -> Field {
    Field {
        id: SHAPE,
        label: crate::ui_text::text("tool-shape"),
        icon: Some(shape_button_icon(index)),
        kind: Kind::Action {
            value: String::new(),
            active: false,
            width: 28,
        },
    }
}
fn number(id: u16, label: &'static str, value: f32, min: i32, max: i32) -> Field {
    field(
        id,
        label,
        Kind::Number {
            value: (value.round() as i32).clamp(min, max),
            min,
            max,
            suffix: if matches!(id, OPACITY | SPOT_DENSITY | LENS_ZOOM) {
                "%"
            } else {
                ""
            },
        },
    )
}
fn action(id: u16, label: &'static str, value: &str, active: bool) -> Field {
    if matches!(id, FONT | DASH | SHAPE) {
        return field(
            id,
            label,
            Kind::Dropdown {
                value: value.into(),
                width: match id {
                    FONT => 110,
                    SHAPE => 56,
                    _ => 46,
                },
            },
        );
    }
    field(
        id,
        label,
        Kind::Action {
            value: value.into(),
            active,
            width: match id {
                FONT => 110,
                DASH => 46,
                _ => 28,
            },
        },
    )
}
fn color(id: u16, label: &'static str, c: Option<Color>, alpha: f32, stroke: bool) -> Field {
    let rgb = if alpha == 0.0 {
        None
    } else {
        c.map(|Color(r, g, b)| ((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    };
    field(id, label, Kind::Color { rgb, stroke })
}
pub(super) fn fields(c: &Canvas) -> Vec<Field> {
    let Some(v) = view(c) else { return Vec::new() };
    let style = v.style;
    let tool = v.tool;
    let profiles = &c.tool_profiles;
    let stroke = || {
        color(
            STROKE,
            "edit-stroke-color",
            Some(style.color()),
            style.paint_alpha().stroke(),
            true,
        )
    };
    let fill = || {
        color(
            FILL,
            "edit-fill-color",
            style.fill(),
            style.paint_alpha().fill(),
            false,
        )
    };
    let width = || number(WIDTH, "edit-line-width", style.width(), 0, 999);
    let dash = || {
        action(
            DASH,
            "edit-line-style",
            match style.dash() {
                crate::stroke_style::DashPattern::Solid => "━",
                crate::stroke_style::DashPattern::Dash => "╌",
                _ => "┄",
            },
            false,
        )
    };
    let mut result = match tool {
        Tool::Magnifier => {
            let lens = v
                .selected
                .and_then(|i| c.editor.elements()[i].lens)
                .unwrap_or(c.tool_profiles.lens);
            let shape_index = match lens.shape {
                crate::editor::LensShape::Rect => 0,
                crate::editor::LensShape::Rounded => 1,
                crate::editor::LensShape::Ellipse => 2,
            };
            let mut f = vec![
                shape_dropdown(shape_index),
                stroke(),
                number(
                    LENS_ZOOM,
                    "tool-lens-zoom",
                    f32::from(lens.percent),
                    150,
                    800,
                ),
            ];
            if lens.shape == crate::editor::LensShape::Rounded {
                let radius = v.selected.map_or_else(
                    || style.corner_radius(),
                    |i| c.editor.elements()[i].corner_radius,
                );
                f.push(number(RADIUS, "edit-corner-radius", radius, 0, 999));
            }
            f
        }
        t if t.spotlight() => {
            let shape_index = match t {
                Tool::SpotRect => 0,
                Tool::SpotRounded => 1,
                _ => 2,
            };
            let mut f = vec![
                shape_dropdown(shape_index),
                number(
                    SPOT_DENSITY,
                    "tool-spot-density",
                    f32::from(c.editor.spotlight_percent()),
                    20,
                    90,
                ),
            ];
            if t == Tool::SpotRounded {
                let radius = v.selected.map_or_else(
                    || style.corner_radius(),
                    |i| c.editor.elements()[i].corner_radius,
                );
                f.push(number(RADIUS, "edit-corner-radius", radius, 0, 999));
            }
            f
        }
        Tool::Rectangle
        | Tool::RoundedRectangle
        | Tool::Ellipse
        | Tool::Triangle
        | Tool::Polygon => {
            let mut f = vec![fill(), stroke(), width()];
            if tool != Tool::Ellipse {
                f.push(number(
                    RADIUS,
                    "edit-corner-radius",
                    style.corner_radius(),
                    0,
                    999,
                ));
            }
            f.push(dash());
            if crate::shape_options::ShapeOptions::supports_ratio(tool) {
                f.push(field(
                    RATIO,
                    "edit-aspect-ratio",
                    Kind::Ratio {
                        value: profiles
                            .shapes
                            .ratio_label()
                            .replace('：', ":")
                            .split(':')
                            .map(str::trim)
                            .collect::<Vec<_>>()
                            .join(":"),
                        active: profiles.shapes.lock_aspect,
                    },
                ));
            } else {
                f.push(action(
                    CONSTRAIN,
                    "edit-regular-polygon",
                    "",
                    profiles.shapes.lock_aspect,
                ));
            }
            f.push(number(
                STEP,
                "edit-step",
                profiles.shapes.step() as f32,
                0,
                999,
            ));
            if tool == Tool::Polygon {
                f.push(number(SIDES, "edit-sides", style.sides() as f32, 3, 12));
            }
            f
        }
        Tool::Line | Tool::Arrow | Tool::Bezier | Tool::Polyline => {
            let mut f = vec![
                stroke(),
                width(),
                dash(),
                action(
                    ARROW,
                    "edit-arrow",
                    "",
                    tool == Tool::Arrow || style.arrow(),
                ),
            ];
            if v.selected.is_none() && matches!(tool, Tool::Line | Tool::Arrow) {
                f.push(action(
                    ANGLE,
                    "edit-angle-snap",
                    "",
                    profiles.angle_snap(tool),
                ));
            }
            f
        }
        Tool::Brush | Tool::Paintbrush | Tool::Highlighter => {
            let mut f = vec![
                color(
                    STROKE,
                    "edit-brush-color",
                    Some(style.color()),
                    style.paint_alpha().stroke(),
                    false,
                ),
                number(WIDTH, "edit-width", style.width(), 1, 999),
            ];
            if tool != Tool::Highlighter {
                f.push(number(
                    OPACITY,
                    "common-opacity",
                    style.opacity() * 100.0,
                    0,
                    100,
                ));
                f.push(number(
                    SMOOTH,
                    "edit-smoothing",
                    profiles.smoothing(tool) as f32,
                    0,
                    100,
                ));
            }
            if tool != Tool::Paintbrush {
                f.push(action(
                    GLOW,
                    "edit-highlighter",
                    "",
                    profiles.brush_highlighter_enabled(),
                ));
            }
            f
        }
        Tool::Text | Tool::Serial => {
            let text = v
                .selected
                .and_then(|i| c.editor.elements()[i].text.as_ref());
            let font = text.map(|t| t.family()).unwrap_or_else(|| {
                if tool == Tool::Serial {
                    profiles.serial_font_family()
                } else {
                    profiles.text_font.family()
                }
            });
            let mut f = vec![fill()];
            f.push(color(
                STROKE,
                if tool == Tool::Serial {
                    "edit-number-color"
                } else {
                    "edit-text-color"
                },
                Some(style.color()),
                style.paint_alpha().stroke(),
                tool == Tool::Serial,
            ));
            let size = if tool == Tool::Serial {
                v.selected
                    .map(|i| {
                        let p = &c.editor.elements()[i].points;
                        (p[1].x - p[0].x).abs() / 2.0
                    })
                    .unwrap_or(profiles.serial_radius() as f32)
            } else {
                text.map_or(profiles.text_font.font_size(), |t| t.font_size())
            };
            f.push(number(
                if tool == Tool::Serial {
                    SERIAL_RADIUS
                } else {
                    SIZE
                },
                if tool == Tool::Serial {
                    "edit-radius"
                } else {
                    "edit-font-size"
                },
                size,
                8,
                999,
            ));
            f.push(action(FONT, "edit-font", font, false));
            if tool == Tool::Text {
                let b = match text {
                    Some(t) => t.geometry(),
                    None => profiles.text_box,
                };
                f.push(action(TEXT_BOX, "edit-text-wrap", "", b.is_some()));
                if let Some(b) = b {
                    f.push(number(BOX_WIDTH, "edit-box-width", b.width(), 8, 16384));
                    f.push(number(BOX_PAD, "edit-box-padding", b.padding(), 0, 256));
                    f.push(number(RADIUS, "edit-radius", style.corner_radius(), 0, 999));
                }
            }
            f
        }
        Tool::Mosaic | Tool::Blur => vec![
            action(EFFECT_RECT, "edit-rect-mask", "", profiles.effect_rect),
            number(WIDTH, "edit-brush-size", style.width(), 4, 200),
            if tool == Tool::Mosaic {
                number(CELL, "edit-mosaic-cell", style.mosaic_cell(), 2, 64)
            } else {
                number(
                    STRENGTH,
                    "edit-blur-strength",
                    profiles.blur_strength(),
                    2,
                    64,
                )
            },
        ],
        Tool::Eraser => vec![
            number(WIDTH, "edit-eraser-size", style.width(), 4, 96),
            action(
                ONLY_MARKS,
                "edit-erase-annotations",
                "",
                c.erase_annotations_only,
            ),
        ],
        _ => Vec::new(),
    };
    // Ordinary text actions do not overlay icons; the theme lays out Dropdown icons, values, and expansion arrows.
    for f in &mut result {
        if matches!(f.kind,Kind::Action{ref value,..} if !value.is_empty()) {
            f.icon = None;
        }
    }
    result
}
