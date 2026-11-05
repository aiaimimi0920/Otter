//! 会话基础矢量导入：局部 Editor 构造成功后交付，未知类型不静默丢失。
use crate::{
    editor::{Editor, Tool},
    stroke_style::{Color, DashPattern, PaintAlpha, StrokeStyle},
    viewport::Point,
};
use serde_json::{value::RawValue, Value};
pub struct Restored {
    pub editor: Editor,
    /// 初始恢复清单；活动编辑后的身份必须读取 Stroke::annotation_id，不能继续按此下标套用。
    pub ids: Vec<String>,
    pub serial_counter: u32,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Shape,
    Budget,
    Unsupported,
    Geometry,
    Style,
    Identity,
}

#[path = "session_vector_color.rs"]
mod color_value;
#[path = "session_vector_text.rs"]
mod text;
pub use text::Metrics as TextMetrics;
fn number(value: &Value) -> Result<f32, Error> {
    let n = value
        .as_f64()
        .filter(|n| n.is_finite() && n.abs() <= 1_000_000.0)
        .ok_or(Error::Geometry)?;
    Ok(n as f32)
}
fn point(value: &Value) -> Result<Point, Error> {
    Ok(Point {
        x: number(&value["x"])?,
        y: number(&value["y"])?,
    })
}
fn color(value: &Value) -> Result<(Color, f32), Error> {
    color_value::parse(value.as_str().ok_or(Error::Style)?).ok_or(Error::Style)
}
fn style(value: &Value, sides: Option<u64>, arrow: bool) -> Result<StrokeStyle, Error> {
    style_impl(value, sides, arrow, false)
}
fn style_impl(
    value: &Value,
    sides: Option<u64>,
    arrow: bool,
    serial: bool,
) -> Result<StrokeStyle, Error> {
    let fill = match value.get("fill") {
        None => None,
        Some(Value::String(v)) if matches!(v.as_str(), "" | "transparent" | "none") => None,
        Some(_) => Some(color(&value["fill"])?),
    };
    let dash = match value.get("dashPattern").and_then(Value::as_str) {
        None | Some("solid") => DashPattern::Solid,
        Some("dash-1") => DashPattern::Dash,
        Some("dash-2") => DashPattern::DashDot,
        _ => return Err(Error::Style),
    };
    let sides = sides.unwrap_or(6);
    if !(3..=64).contains(&sides) {
        return Err(Error::Style);
    }
    let (color, alpha) = color(&value["color"])?;
    let paint_alpha = PaintAlpha::new(alpha, fill.map_or(1.0, |v| v.1)).ok_or(Error::Style)?;
    StrokeStyle::restored(number(&value["width"])?, color)
        .ok_or(Error::Style)?
        .with_opacity(value.get("opacity").map(number).transpose()?.unwrap_or(1.0))
        .ok_or(Error::Style)?
        .with_corner_radius_value(if serial {
            0.0
        } else {
            value
                .get("cornerRadius")
                .map(number)
                .transpose()?
                .unwrap_or(0.0)
        })
        .ok_or(Error::Style)
        .map(|style| {
            style
                .with_fill(fill.map(|v| v.0))
                .with_paint_alpha(paint_alpha)
                .with_dash(dash)
                .with_sides_value(sides as u8)
                .expect("validated sides")
                .with_arrow(arrow)
        })
}

/// 无平台测量入口支持形状/路径/序号；普通文字需使用 restore_with_text 注入真实字体测量。
pub fn restore(raw: Option<&RawValue>) -> Result<Restored, Error> {
    restore_with_text(raw, |_| Err(Error::Unsupported))
}
pub fn restore_with_text(
    raw: Option<&RawValue>,
    measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<TextMetrics, Error>,
) -> Result<Restored, Error> {
    restore_with_resources(raw, measure, |_| Err(Error::Unsupported))
}
/// blur 提供方必须按当前恢复底图/坐标框计算资源，可共享相同强度结果；不得联网或返回旧文档纹理。
pub fn restore_with_resources(
    raw: Option<&RawValue>,
    mut measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<TextMetrics, Error>,
    mut blur: impl FnMut(f32) -> Result<crate::blur_image::BlurImage, Error>,
) -> Result<Restored, Error> {
    let Some(raw) = raw else {
        return Ok(Restored {
            editor: Editor::default(),
            ids: Vec::new(),
            serial_counter: 0,
        });
    };
    if raw.get().len() > 8 * 1024 * 1024 {
        return Err(Error::Budget);
    }
    let value: Value = serde_json::from_str(raw.get()).map_err(|_| Error::Shape)?;
    let elements = value["elements"].as_array().ok_or(Error::Shape)?;
    if elements.len() > 4096 {
        return Err(Error::Budget);
    }
    let serial_counter = value["serialCounter"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(Error::Shape)?;
    let mut ordered = Vec::with_capacity(elements.len());
    let mut seen = std::collections::HashSet::new();
    for element in elements {
        let id = element["id"]
            .as_str()
            .filter(|id| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control))
            .ok_or(Error::Identity)?;
        if !seen.insert(id) {
            return Err(Error::Identity);
        }
        let z = element["zIndex"]
            .as_f64()
            .filter(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            .ok_or(Error::Geometry)?;
        ordered.push((z, id, element));
    }
    ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
    let layers = ordered.iter().map(|v| v.0).collect::<Vec<_>>();
    let mut editor = Editor::default();
    let mut ids = Vec::with_capacity(ordered.len());
    for (_, id, element) in ordered {
        let kind = element["type"].as_str().ok_or(Error::Shape)?;
        if matches!(kind, "text" | "serial") {
            text::append(&mut editor, element, kind == "serial", &mut measure)?;
            ids.push(id.to_owned());
            continue;
        }
        let (tool, shape) = match kind {
            "rect" => (Tool::Rectangle, true),
            "round-rect" => (Tool::RoundedRectangle, true),
            "ellipse" => (Tool::Ellipse, true),
            "triangle" => (Tool::Triangle, true),
            "polygon" => (Tool::Polygon, true),
            "line" => (Tool::Line, false),
            "arrow" => (Tool::Arrow, false),
            "polyline" => (Tool::Polyline, false),
            "bezier" => (Tool::Bezier, false),
            "mosaic" => (Tool::Mosaic, false),
            "blur" => (Tool::Blur, false),
            "brush" => (
                match element["style"].get("brushTip").and_then(Value::as_str) {
                    Some("tapered") => Tool::Paintbrush,
                    None => Tool::Brush,
                    _ => return Err(Error::Unsupported),
                },
                false,
            ),
            "highlighter" => (Tool::Highlighter, false),
            _ => return Err(Error::Unsupported),
        };
        if tool != Tool::Paintbrush
            && (element["style"].get("brushTip").is_some()
                || element["style"].get("brushWeights").is_some())
        {
            return Err(Error::Unsupported);
        }
        let weights = if tool == Tool::Paintbrush {
            match element["style"].get("brushWeights") {
                None => Vec::new(),
                Some(value) => {
                    let values = value.as_array().ok_or(Error::Shape)?;
                    if values.len() > 65_536 {
                        return Err(Error::Budget);
                    }
                    values.iter().map(number).collect::<Result<Vec<_>, _>>()?
                }
            }
        } else {
            Vec::new()
        };
        let points = if shape {
            let a = point(element)?;
            let w = number(&element["w"])?;
            let h = number(&element["h"])?;
            if w < 0.0 || h < 0.0 {
                return Err(Error::Geometry);
            }
            vec![
                a,
                Point {
                    x: a.x + w,
                    y: a.y + h,
                },
            ]
        } else {
            let points = element["points"].as_array().ok_or(Error::Shape)?;
            if points.is_empty() || points.len() > 65_536 {
                return Err(Error::Budget);
            }
            points.iter().map(point).collect::<Result<Vec<_>, _>>()?
        };
        let mut restored_style = style(
            &element["style"],
            element["sides"].as_u64(),
            element["showArrowHead"].as_bool().unwrap_or(false),
        )?;
        if matches!(tool, Tool::Mosaic | Tool::Blur) {
            let width = element
                .get("brushWidth")
                .map(number)
                .transpose()?
                .unwrap_or(restored_style.width())
                .max(1.0);
            let strength = element
                .get("strength")
                .map(number)
                .transpose()?
                .filter(|value| *value != 0.0)
                .unwrap_or(12.0)
                .round()
                .max(2.0);
            restored_style = restored_style.with_width(width).ok_or(Error::Style)?;
            if tool == Tool::Mosaic {
                restored_style = restored_style
                    .with_mosaic_cell_value(strength)
                    .ok_or(Error::Style)?;
            }
        }
        editor.set_style(restored_style);
        let rotation = element
            .get("rotation")
            .map(number)
            .transpose()?
            .unwrap_or(0.0);
        let result = if tool == Tool::Blur {
            let strength = element
                .get("strength")
                .map(number)
                .transpose()?
                .filter(|value| *value != 0.0)
                .unwrap_or(8.0);
            if !(1.0..=64.0).contains(&strength) {
                return Err(Error::Style);
            }
            let image = blur(strength)?;
            if image.strength() != strength {
                return Err(Error::Style);
            }
            editor.append_restored_blur(points, rotation, image)
        } else {
            editor.append_restored(tool, points, rotation, weights)
        };
        result.map_err(|error| match error {
            crate::editor::EditError::BudgetExceeded => Error::Budget,
            _ => Error::Geometry,
        })?;
        ids.push(id.to_owned());
    }
    Ok(Restored {
        editor: editor
            .finish_restore(serial_counter)
            .with_restored_annotation_ids(&ids)
            .and_then(|editor| editor.with_restored_annotation_layers(&layers))
            .map_err(|_| Error::Identity)?,
        ids,
        serial_counter,
    })
}
#[cfg(test)]
#[path = "session_vector_alpha_tests.rs"]
mod alpha_tests;
#[cfg(test)]
#[path = "session_vector_path_tests.rs"]
mod path_tests;
#[cfg(test)]
#[path = "session_vectors_tests.rs"]
mod tests;
