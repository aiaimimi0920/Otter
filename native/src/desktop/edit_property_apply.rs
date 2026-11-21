//! 字段只修改自己的样式，单选字段作用于已选对象，不泄漏到后续创建默认值。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, DashPattern, StrokeStyle},
    text_annotation::TextFont,
};
use windows::core::{Error, Result};
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid edit property value")
}
fn number(raw: Option<&str>, min: i32, max: i32) -> Result<f32> {
    let v = raw
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|v| (min..=max).contains(v))
        .ok_or_else(invalid)?;
    Ok(v as f32)
}
pub(super) fn restyle(c: &mut Canvas, index: Option<usize>, style: StrokeStyle) {
    if let Some(i) = index {
        if c.editor.restyle(i, style) {
            c.selected = Some((i, c.editor.revision()));
        }
    } else {
        c.editor.set_style(style);
    }
}
#[cfg(test)]
pub(super) fn color(c: &mut Canvas, fill: bool, color: Option<Color>) {
    let color = color
        .map(|Color(r, g, b)| neuro_mini_ui::Rgba::rgb(r, g, b))
        .unwrap_or(neuro_mini_ui::Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    rgba(c, fill, color);
}
pub(super) fn rgba(c: &mut Canvas, fill: bool, color: neuro_mini_ui::Rgba) {
    let Some(v) = fields::view(c) else { return };
    let old = v.style.paint_alpha();
    let alpha = color.a as f32 / 255.0;
    let style = if fill {
        v.style
            .with_fill((color.a != 0).then_some(Color(color.r, color.g, color.b)))
            .with_paint_alpha(crate::stroke_style::PaintAlpha::new(old.stroke(), alpha).unwrap())
    } else {
        v.style
            .with_color(if color.a == 0 {
                v.style.color()
            } else {
                Color(color.r, color.g, color.b)
            })
            .with_paint_alpha(crate::stroke_style::PaintAlpha::new(alpha, old.fill()).unwrap())
    };
    restyle(c, v.selected, style);
}
pub(super) unsafe fn font(c: &mut Canvas, family: &str, size: Option<f32>) -> Result<()> {
    let Some(v) = fields::view(c) else {
        return Ok(());
    };
    if let Some(i) = v.selected {
        let Some(pending) = super::super::text_input::Pending::selected(c) else {
            return Ok(());
        };
        let t = pending.text().unwrap();
        let font =
            TextFont::new(family, size.unwrap_or(t.font_size()), t.bold()).ok_or_else(invalid)?;
        let text = t.with_font(font);
        let measured = super::super::annotation_text::measure(&text)?;
        pending.commit(c, text, measured)?;
        c.selected = Some((i, c.editor.revision()));
    } else if v.tool == Tool::Serial {
        if !c.tool_profiles.set_serial_font_family(family) {
            return Err(invalid());
        }
    } else if v.tool == Tool::Text {
        let font = &c.tool_profiles.text_font;
        c.tool_profiles.text_font =
            TextFont::new(family, size.unwrap_or(font.font_size()), font.bold())
                .ok_or_else(invalid)?;
    }
    Ok(())
}
pub(super) unsafe fn value(c: &mut Canvas, id: u16, raw: Option<&str>) -> Result<()> {
    let Some(v) = fields::view(c) else {
        return Ok(());
    };
    let style = v.style;
    let tool = v.tool;
    let next = match id {
        WIDTH => {
            let (min, max) = match tool {
                Tool::Mosaic | Tool::Blur => (4, 200),
                Tool::Eraser => (4, 96),
                Tool::Brush | Tool::Paintbrush | Tool::Highlighter => (1, 999),
                _ => (0, 999),
            };
            style.with_width_value(number(raw, min, max)?)
        }
        RADIUS => style.with_corner_radius_value(number(raw, 0, 999)?),
        SIDES => Some(style.with_sides(number(raw, 3, 12)? as u8)),
        OPACITY => style.with_opacity(number(raw, 0, 100)? / 100.0),
        CELL => style.with_mosaic_cell_value(number(raw, 2, 64)?),
        ARROW => {
            let enabled = !(tool == Tool::Arrow || style.arrow());
            if let Some(i) = v.selected {
                c.editor
                    .set_line_arrow(i, enabled)
                    .map_err(|e| Error::new(E_FAIL, format!("Arrow property: {e:?}")))?;
                c.selected = Some((i, c.editor.revision()));
                return Ok(());
            }
            if tool == Tool::Arrow {
                c.select_tool(Tool::Line);
            }
            Some(c.editor.style().with_arrow(enabled))
        }
        CONSTRAIN => {
            c.tool_profiles.shapes.lock_aspect = !c.tool_profiles.shapes.lock_aspect;
            return Ok(());
        }
        RATIO => {
            if let Some(raw) = raw {
                if !c.tool_profiles.shapes.set_ratio(raw) {
                    return Err(invalid());
                }
                c.tool_profiles.shapes.lock_aspect = true;
            } else {
                c.tool_profiles.shapes.lock_aspect = !c.tool_profiles.shapes.lock_aspect;
            }
            return Ok(());
        }
        STEP => {
            if !c.tool_profiles.shapes.set_step(raw.unwrap_or("")) {
                return Err(invalid());
            }
            return Ok(());
        }
        ANGLE => {
            c.tool_profiles.toggle_angle_snap(tool);
            return Ok(());
        }
        SMOOTH => {
            c.tool_profiles
                .set_smoothing(tool, number(raw, 0, 100)? as u8);
            return Ok(());
        }
        STRENGTH => {
            c.tool_profiles.set_blur_strength(number(raw, 2, 64)?);
            c.blur_ready = None;
            return Ok(());
        }
        SERIAL_RADIUS => {
            let radius = number(raw, 8, 999)?;
            if let Some(i) = v.selected {
                let p = &c.editor.elements()[i].points;
                let current = (p[1].x - p[0].x).abs() / 2.0;
                c.editor
                    .resize(i, radius / current)
                    .map_err(|e| Error::new(E_FAIL, format!("Serial radius: {e:?}")))?;
                c.selected = Some((i, c.editor.revision()));
            } else {
                c.tool_profiles.set_serial_radius(radius as u16);
            }
            return Ok(());
        }
        SIZE => {
            let size = number(raw, 8, 999)?;
            let family = v
                .selected
                .and_then(|i| c.editor.elements()[i].text.as_ref())
                .map(|t| t.family())
                .unwrap_or(c.tool_profiles.text_font.family())
                .to_owned();
            font(c, &family, Some(size))?;
            return Ok(());
        }
        GLOW => {
            c.tool_profiles.toggle_brush_highlighter();
            return Ok(());
        }
        ONLY_MARKS => {
            c.erase_annotations_only = !c.erase_annotations_only;
            return Ok(());
        }
        DASH => {
            let dash = match raw {
                Some("0") => DashPattern::Solid,
                Some("1") => DashPattern::Dash,
                Some("2") => DashPattern::DashDot,
                _ => return Err(invalid()),
            };
            Some(style.with_dash(dash))
        }
        _ => return Err(invalid()),
    }
    .ok_or_else(invalid)?;
    restyle(c, v.selected, next);
    Ok(())
}
