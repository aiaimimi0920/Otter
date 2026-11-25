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
    } else {
        None
    };
    let (tool, style) = if let Some(index) = selected {
        let s = &c.editor.elements()[index];
        if !matches!(
            s.tool,
            Tool::Text | Tool::Serial | Tool::Line | Tool::Arrow | Tool::Bezier
        ) {
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
        _ => None,
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
            suffix: if id == OPACITY { "%" } else { "" },
        },
    )
}
fn action(id: u16, label: &'static str, value: &str, active: bool) -> Field {
    if matches!(id, FONT | DASH) {
        return field(
            id,
            label,
            Kind::Dropdown {
                value: value.into(),
                width: if id == FONT { 110 } else { 46 },
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
            f
        }
        Tool::Mosaic | Tool::Blur => vec![
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
