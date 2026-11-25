//! 将主干文字基线/序号圆心换算为原生布局框；字体测量由平台注入，不猜测字宽。
use super::*;
use crate::text_annotation::TextAnnotation;
pub struct Metrics {
    pub size: Point,
    pub baseline: f32,
}
pub(super) fn append(
    editor: &mut Editor,
    element: &Value,
    serial: bool,
    measure: &mut impl FnMut(&TextAnnotation) -> Result<Metrics, Error>,
) -> Result<(), Error> {
    let anchor = point(element)?;
    let radius = element["style"]
        .get("cornerRadius")
        .map(number)
        .transpose()?
        .unwrap_or(14.0);
    if !(0.0..=1_000_000.0).contains(&radius) {
        return Err(Error::Style);
    }
    let font_size = element
        .get("fontSize")
        .map(number)
        .transpose()?
        .unwrap_or(if serial {
            (radius.round().clamp(8.0, 999.0) * 1.15).round().max(10.0)
        } else {
            18.0
        });
    let family = element
        .get("fontFamily")
        .and_then(Value::as_str)
        .unwrap_or("Segoe UI");
    let text = TextAnnotation::restored(
        element["text"].as_str().ok_or(Error::Shape)?,
        family,
        font_size,
        serial,
    )
    .ok_or(Error::Style)?;
    let (start, size) = if serial {
        (
            Point {
                x: anchor.x,
                y: anchor.y - font_size / 2.0 - radius,
            },
            Point {
                x: radius * 2.0,
                y: radius * 2.0,
            },
        )
    } else {
        let metrics = measure(&text)?;
        if !metrics.size.x.is_finite()
            || !metrics.size.y.is_finite()
            || metrics.size.x <= 0.0
            || metrics.size.y <= 0.0
            || !metrics.baseline.is_finite()
            || metrics.baseline < 0.0
        {
            return Err(Error::Geometry);
        }
        (
            Point {
                x: anchor.x - 4.0,
                y: anchor.y - metrics.baseline,
            },
            metrics.size,
        )
    };
    // 序号的 cornerRadius 是圆半径，不是矩形圆角，不能受普通形状的 256 上限限制。
    editor.set_style(style_impl(&element["style"], None, false, serial)?);
    let rotation = element
        .get("rotation")
        .map(number)
        .transpose()?
        .unwrap_or(0.0);
    editor
        .append_restored_text(
            vec![
                start,
                Point {
                    x: start.x + size.x,
                    y: start.y + size.y,
                },
            ],
            rotation,
            text,
            serial,
        )
        .map_err(|_| Error::Geometry)
}
