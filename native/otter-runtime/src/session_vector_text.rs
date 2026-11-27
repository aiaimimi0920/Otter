//! Convert baseline text baselines/sequence-circle centers into native layout boxes; platform-injected font measurement avoids guessing character widths.
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
    let boxed = element["type"] == "text-box";
    if !boxed && (element.get("boxWidth").is_some() || element.get("boxPadding").is_some()) {
        return Err(Error::Unsupported);
    }
    let geometry = if boxed {
        Some(
            crate::text_annotation::TextBox::new(
                number(&element["boxWidth"])?,
                number(&element["boxPadding"])?,
            )
            .ok_or(Error::Geometry)?,
        )
    } else {
        None
    };
    let text = TextAnnotation::restored(
        element["text"].as_str().ok_or(Error::Shape)?,
        family,
        font_size,
        if boxed || element["type"] == "serial-arrow" {
            element["bold"].as_bool().ok_or(Error::Style)?
        } else {
            serial
        },
    )
    .ok_or(Error::Style)?
    .with_geometry(geometry);
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
                x: anchor.x - text.padding(),
                y: anchor.y - metrics.baseline,
            },
            metrics.size,
        )
    };
    // Sequence-label cornerRadius is a circle radius, not rectangle corner rounding, and must not use ordinary shapes' 256 limit.
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
        .map_err(|_| Error::Geometry)?;
    if element["type"] == "serial-arrow" {
        editor
            .restore_serial_tip(point(&element["serialArrowTip"])?)
            .map_err(|_| Error::Geometry)?;
    } else if element.get("serialArrowTip").is_some() {
        return Err(Error::Unsupported);
    }
    Ok(())
}
