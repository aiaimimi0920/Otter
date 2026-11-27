//! Write native annotations in mainline format; merge extension fields by stable ID without touching disk or the current editor.
use crate::{
    editor::{Editor, Stroke, Tool},
    stroke_style::{Color, DashPattern},
    viewport::Point,
};
use serde_json::{json, value::RawValue, Map, Value};
#[path = "session_vector_write_frame.rs"]
pub mod frame;
pub use frame::{encode_visible, Visible};
#[path = "session_vector_write_mapping.rs"]
pub mod mapping;
pub use mapping::Mapping;
#[path = "session_vector_write_order.rs"]
pub mod order;
pub use order::Plan as OrderPlan;

const LIMIT: usize = 8 * 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Shape,
    Identity,
    Geometry,
    Unsupported,
    Budget,
}

/// Callers provide baselines measured in the same font environment as import; drafts never enter persistent snapshots.
/// This layer emits Editor coordinates; session wrappers must save matching source-frame/crop mappings rather than directly replacing legacy cropped-session fields.
pub fn encode(
    editor: &Editor,
    original: Option<&RawValue>,
    baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
) -> Result<Vec<u8>, Error> {
    encode_origin(editor, original, Point::default(), baseline)
}

fn encode_origin(
    editor: &Editor,
    original: Option<&RawValue>,
    origin: Point,
    baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
) -> Result<Vec<u8>, Error> {
    encode_impl(editor, original, origin, None, baseline, &|| false)
}
/// Emit ordinary-viewport annotations in session logical units; mapping already includes crop/translation, so never subtract visible-frame origin again.
/// Minimized windows use different image and annotation viewports; callers must not pass their display viewport directly here.
pub fn encode_mapped(
    editor: &Editor,
    original: Option<&RawValue>,
    mapping: Mapping,
    baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
) -> Result<Vec<u8>, Error> {
    encode_mapped_cancellable(editor, original, mapping, baseline, || false)
}
/// Background save entry point; callbacks observe cancellation only, without changing editors or publishing output.
pub fn encode_mapped_cancellable(
    editor: &Editor,
    original: Option<&RawValue>,
    mapping: Mapping,
    baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<u8>, Error> {
    encode_impl(
        editor,
        original,
        Point::default(),
        Some(mapping),
        baseline,
        &cancelled,
    )
}
fn encode_impl(
    editor: &Editor,
    original: Option<&RawValue>,
    origin: Point,
    mapping: Option<Mapping>,
    mut baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<u8>, Error> {
    check(cancelled)?;
    finite(origin.x)?;
    finite(origin.y)?;
    let mut root = match original {
        Some(raw) if raw.get().len() <= LIMIT => {
            serde_json::from_str::<Value>(raw.get()).map_err(|_| Error::Shape)?
        }
        Some(_) => return Err(Error::Budget),
        None => json!({}),
    };
    let root = root.as_object_mut().ok_or(Error::Shape)?;
    if editor.elements().len() > 4096 {
        return Err(Error::Budget);
    }
    let old_elements = match root.get("elements") {
        Some(Value::Array(elements)) if elements.len() <= 4096 => Some(elements),
        Some(Value::Array(_)) => return Err(Error::Budget),
        Some(_) => return Err(Error::Shape),
        None => None,
    };
    let order = OrderPlan::new(editor, old_elements.into_iter().flatten());
    let mut previous = std::collections::HashMap::new();
    if let Some(elements) = root.remove("elements") {
        let Value::Array(elements) = elements else {
            return Err(Error::Shape);
        };
        for element in elements {
            check(cancelled)?;
            let id = element
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control))
                .ok_or(Error::Identity)?
                .to_owned();
            if previous.insert(id, element).is_some() {
                return Err(Error::Identity);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut elements = Vec::with_capacity(editor.elements().len());
    for (index, stroke) in editor.elements().iter().enumerate() {
        check(cancelled)?;
        if !seen.insert(stroke.annotation_id()) {
            return Err(Error::Identity);
        }
        let old = previous
            .remove(stroke.annotation_id())
            .unwrap_or_else(|| json!({}));
        let mut value = element(stroke, index, old, &mut baseline, cancelled)?;
        value["zIndex"] = order.value(stroke.annotation_id(), index);
        frame::translate(&mut value, origin)?;
        if let Some(mapping) = mapping {
            mapping.apply(stroke, &mut value, &mut baseline, cancelled)?;
        }
        elements.push(value);
    }
    root.insert("elements".into(), elements.into());
    root.insert("serialCounter".into(), json!(editor.serial_count()));
    if editor.elements().iter().any(|s| s.tool.spotlight()) || editor.spotlight_percent() != 60 {
        root.insert("spotlightPercent".into(), json!(editor.spotlight_percent()));
    } else {
        root.remove("spotlightPercent");
    }
    let mut output = Bounded(Vec::new());
    let mut sink = Cancellable {
        output: &mut output,
        cancelled,
    };
    serde_json::to_writer(&mut sink, root).map_err(|_| {
        if cancelled() {
            Error::Cancelled
        } else {
            Error::Budget
        }
    })?;
    check(cancelled)?;
    Ok(output.0)
}

fn element(
    stroke: &Stroke,
    index: usize,
    old: Value,
    baseline: &mut impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
    cancelled: &impl Fn() -> bool,
) -> Result<Value, Error> {
    let Value::Object(mut object) = old else {
        return Err(Error::Shape);
    };
    let kind = match stroke.tool {
        Tool::Rectangle => "rect",
        Tool::SpotRect => "spotlight-rect",
        Tool::Magnifier => "magnifier",
        Tool::SpotRounded => "spotlight-round",
        Tool::SpotEllipse => "spotlight-ellipse",
        Tool::RoundedRectangle => "round-rect",
        Tool::Ellipse => "ellipse",
        Tool::Triangle => "triangle",
        Tool::Polygon => "polygon",
        Tool::Line => "line",
        Tool::Arrow => "arrow",
        Tool::Bezier => "bezier",
        Tool::Polyline => "polyline",
        Tool::Brush | Tool::Paintbrush => "brush",
        Tool::Highlighter => "highlighter",
        Tool::Text if stroke.text.as_ref().and_then(|t| t.geometry()).is_some() => "text-box",
        Tool::Text => "text",
        Tool::Serial if stroke.serial_arrow_tip.is_some() => "serial-arrow",
        Tool::Serial => "serial",
        Tool::Mosaic if stroke.effect_rect => "mosaic-rect",
        Tool::Blur if stroke.effect_rect => "blur-rect",
        Tool::Mosaic => "mosaic",
        Tool::Blur => "blur",
        _ => return Err(Error::Unsupported),
    };
    // Clear owned fields so removing fill or switching tools cannot revive old parameters; preserve unknown extensions unchanged.
    for key in [
        "x",
        "y",
        "w",
        "h",
        "points",
        "rotation",
        "sides",
        "showArrowHead",
        "text",
        "fontSize",
        "fontFamily",
        "strength",
        "brushWidth",
        "serialArrowTip",
        "sourceOffset",
        "zoomTenths",
        "zoomPercent",
    ] {
        object.remove(key);
    }
    object.insert("id".into(), stroke.annotation_id().into());
    object.insert("type".into(), kind.into());
    if kind == "text-box" || object.contains_key("boxWidth") {
        for key in ["boxWidth", "boxPadding", "bold"] {
            object.remove(key);
        }
    }
    object.insert("zIndex".into(), json!(index));
    let mut style = match object.remove("style") {
        Some(Value::Object(style)) => style,
        None => Map::new(),
        _ => return Err(Error::Shape),
    };
    for key in ["brushTip", "brushWeights", "fill"] {
        style.remove(key);
    }
    style.insert(
        "color".into(),
        color(stroke.color, stroke.paint_alpha.stroke()).into(),
    );
    style.insert("width".into(), finite(stroke.width)?);
    style.insert("opacity".into(), finite(stroke.opacity)?);
    style.insert("cornerRadius".into(), finite(stroke.corner_radius)?);
    style.insert(
        "dashPattern".into(),
        match stroke.dash {
            DashPattern::Solid => "solid",
            DashPattern::Dash => "dash-1",
            DashPattern::DashDot => "dash-2",
        }
        .into(),
    );
    if let Some(fill) = stroke.fill {
        style.insert("fill".into(), color(fill, stroke.paint_alpha.fill()).into());
    }
    if stroke.tool == Tool::Paintbrush {
        style.insert("brushTip".into(), "tapered".into());
        style.insert(
            "brushWeights".into(),
            Value::Array(
                stroke
                    .brush_weights
                    .iter()
                    .copied()
                    .map(|weight| {
                        check(cancelled)?;
                        finite(weight)
                    })
                    .collect::<Result<_, _>>()?,
            ),
        );
    }
    let (a, b) = bounds_checked(&stroke.points, cancelled)?;
    if matches!(stroke.tool, Tool::Text | Tool::Serial) {
        let text = stroke.text.as_ref().ok_or(Error::Shape)?;
        if let Some(tip) = stroke.serial_arrow_tip {
            object.insert(
                "serialArrowTip".into(),
                json!({"x":finite(tip.x)?,"y":finite(tip.y)?}),
            );
            object.insert("bold".into(), text.bold().into());
        }
        if let Some(b) = text.geometry() {
            if stroke.tool != Tool::Text {
                return Err(Error::Unsupported);
            }
            object.insert("boxWidth".into(), finite(b.width())?);
            object.insert("boxPadding".into(), finite(b.padding())?);
            object.insert("bold".into(), text.bold().into());
        } else if stroke.tool == Tool::Text && text.bold() {
            return Err(Error::Unsupported);
        }
        let (x, y) = if stroke.tool == Tool::Serial {
            let radius = (b.x - a.x) / 2.0;
            if (b.y - a.y - radius * 2.0).abs() > 0.001 {
                return Err(Error::Geometry);
            }
            if !(0.0..=1_000_000.0).contains(&radius) {
                return Err(Error::Unsupported);
            }
            style.insert("cornerRadius".into(), finite(radius)?);
            (a.x, a.y + text.font_size() / 2.0 + radius)
        } else {
            let base = baseline(text)?;
            if !base.is_finite() || base < 0.0 {
                return Err(Error::Geometry);
            }
            (a.x + text.padding(), a.y + base)
        };
        object.insert("x".into(), finite(x)?);
        object.insert("y".into(), finite(y)?);
        object.insert("text".into(), text.content().into());
        object.insert("fontSize".into(), finite(text.font_size())?);
        object.insert("fontFamily".into(), text.family().into());
        object.insert("rotation".into(), finite(stroke.rotation)?);
    } else if stroke.tool.spotlight()
        || stroke.tool == Tool::Magnifier
        || matches!(
            stroke.tool,
            Tool::Rectangle
                | Tool::RoundedRectangle
                | Tool::Ellipse
                | Tool::Triangle
                | Tool::Polygon
        )
    {
        rect(&mut object, a, b)?;
        if stroke.tool == Tool::Magnifier {
            let lens = stroke.lens.ok_or(Error::Shape)?;
            object.insert(
                "sourceOffset".into(),
                json!({"x":finite(lens.offset.x)?,"y":finite(lens.offset.y)?}),
            );
            if lens.percent % 10 == 0 {
                object.insert("zoomTenths".into(), json!(lens.percent / 10));
            } else {
                object.insert("zoomPercent".into(), json!(lens.percent));
            }
        }
        object.insert("rotation".into(), finite(stroke.rotation)?);
        if stroke.tool == Tool::Polygon {
            object.insert("sides".into(), json!(stroke.sides));
        }
    } else {
        let effect = matches!(stroke.tool, Tool::Mosaic | Tool::Blur);
        let center = crate::rotation::center(&stroke.points);
        let points = stroke
            .points
            .iter()
            .map(|point| {
                check(cancelled)?;
                let point = if effect {
                    *point
                } else {
                    crate::rotation::rotate(*point, center, stroke.rotation)
                };
                Ok(json!({"x": finite(point.x)?, "y": finite(point.y)?}))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        object.insert("points".into(), points.into());
        if effect {
            let pad = if stroke.effect_rect {
                0.0
            } else {
                stroke.width / 2.0
            };
            rect(
                &mut object,
                Point {
                    x: a.x - pad,
                    y: a.y - pad,
                },
                Point {
                    x: b.x + pad,
                    y: b.y + pad,
                },
            )?;
            object.insert("brushWidth".into(), finite(stroke.width)?);
            object.insert("rotation".into(), finite(stroke.rotation)?);
            let strength = if stroke.tool == Tool::Blur {
                stroke.blur.as_ref().ok_or(Error::Shape)?.strength()
            } else {
                // Mainline reads round strength to an integer; reject rather than silently change fractional native-transformed values.
                if !stroke.effect_rect
                    && (stroke.mosaic_cell < 2.0 || stroke.mosaic_cell.fract() != 0.0)
                {
                    return Err(Error::Unsupported);
                }
                stroke.mosaic_cell
            };
            object.insert("strength".into(), finite(strength)?);
        } else {
            object.insert(
                "showArrowHead".into(),
                (stroke.arrow || stroke.tool == Tool::Arrow).into(),
            );
        }
    }
    object.insert("style".into(), style.into());
    Ok(object.into())
}
fn color(Color(r, g, b): Color, alpha: f32) -> String {
    if alpha == 1.0 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        // Use shortest-round-trip f32 decimals; never quantize independent alpha to 8 bits or fold it into overall opacity.
        format!("rgba({r},{g},{b},{alpha})")
    }
}
fn finite(value: f32) -> Result<Value, Error> {
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return Err(Error::Geometry);
    }
    Ok(json!(value))
}
fn bounds(points: &[Point]) -> Result<(Point, Point), Error> {
    bounds_checked(points, &|| false)
}
fn bounds_checked(
    points: &[Point],
    cancelled: &impl Fn() -> bool,
) -> Result<(Point, Point), Error> {
    let mut a = *points.first().ok_or(Error::Geometry)?;
    let mut b = a;
    for p in points {
        check(cancelled)?;
        finite(p.x)?;
        finite(p.y)?;
        a.x = a.x.min(p.x);
        a.y = a.y.min(p.y);
        b.x = b.x.max(p.x);
        b.y = b.y.max(p.y);
    }
    Ok((a, b))
}
fn rect(object: &mut Map<String, Value>, a: Point, b: Point) -> Result<(), Error> {
    if b.x < a.x || b.y < a.y {
        return Err(Error::Geometry);
    }
    for (key, value) in [("x", a.x), ("y", a.y), ("w", b.x - a.x), ("h", b.y - a.y)] {
        object.insert(key.into(), finite(value)?);
    }
    Ok(())
}
struct Bounded(Vec<u8>);
fn check(cancelled: &impl Fn() -> bool) -> Result<(), Error> {
    if cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
struct Cancellable<'a, F> {
    output: &'a mut Bounded,
    cancelled: &'a F,
}
impl<F: Fn() -> bool> std::io::Write for Cancellable<'_, F> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if (self.cancelled)() {
            return Err(std::io::Error::other("Annotation serialization cancelled"));
        }
        self.output.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > LIMIT.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("Annotation output budget exceeded"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_vector_write_cancel_tests.rs"]
mod cancel_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_vector_write_tests.rs"]
mod tests;
