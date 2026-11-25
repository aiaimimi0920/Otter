//! Transform basis-axis geometry and scalar styles separately; inverse transforms divide by forward mean scale, not average reciprocals.
use super::*;
pub(super) fn font_defaults(objects: &mut Objects) {
    for object in objects.values_mut() {
        font_default(object);
    }
}
fn default_size(object: &Value) -> f64 {
    if object["type"] == "serial" {
        let radius = object["style"]["cornerRadius"]
            .as_f64()
            .unwrap_or(14.0)
            .round()
            .clamp(8.0, 999.0);
        (radius * 1.15).round().max(10.0)
    } else {
        18.0
    }
}
fn font_default(object: &mut Value) {
    if !matches!(object["type"].as_str(), Some("text" | "serial"))
        || object.get("fontSize").is_some()
    {
        return;
    }
    let size = default_size(object);
    if let Some(object) = object.as_object_mut() {
        object.insert("fontSize".into(), size.into());
    }
}
#[derive(Clone, Copy)]
struct Scale {
    x: f64,
    y: f64,
    scalar: f64,
    inverse: bool,
}
impl Scale {
    fn new(basis: &Basis, size: Size, inverse: bool) -> Self {
        let (w, h) = basis.size();
        let x = size.width / f64::from(w);
        let y = size.height / f64::from(h);
        Self {
            x,
            y,
            scalar: (x + y) / 2.0,
            inverse,
        }
    }
}
fn field(
    value: &mut Value,
    key: &str,
    scale: f64,
    inverse: bool,
    previous: Option<(&Value, &Value)>,
    optional: bool,
) -> Result<(), Error> {
    let Some(local) = value.get(key) else {
        return if optional {
            Ok(())
        } else {
            Err(Error::Annotation)
        };
    };
    let number = local
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or(Error::Annotation)?;
    // Preserve original f64/integer values for unchanged fields, so changing x cannot round unchanged extreme stroke widths beyond wire gates.
    if let Some((original, expected)) = previous {
        if let (Some(original), Some(expected)) = (original.get(key), expected.get(key)) {
            if edit_equal(local, expected) {
                value[key] = original.clone();
                return Ok(());
            }
        }
    }
    let mapped = if inverse {
        number / scale
    } else {
        number * scale
    };
    if !mapped.is_finite() || (number != 0.0 && mapped == 0.0) {
        return Err(Error::Annotation);
    }
    value[key] = mapped.into();
    Ok(())
}
fn object(value: &Value, scale: Scale, previous: Option<(&Value, &Value)>) -> Result<Value, Error> {
    let kind = value["type"].as_str().ok_or(Error::Annotation)?;
    let previous = previous.filter(|(_, expected)| expected["type"].as_str() == Some(kind));
    if let Some((original, expected)) = previous {
        if edit_equal(value, expected) {
            return Ok(original.clone());
        }
    }
    let mut result = value.clone();
    if !scale.inverse {
        font_default(&mut result);
    }
    let points = matches!(
        kind,
        "line" | "polyline" | "arrow" | "brush" | "highlighter"
    );
    if points {
        let points = result["points"].as_array_mut().ok_or(Error::Annotation)?;
        if points.is_empty() || points.len() > 2048 {
            return Err(Error::Annotation);
        }
        for (index, point) in points.iter_mut().enumerate() {
            let prior = previous
                .and_then(|(o, e)| Some((o["points"].get(index)?, e["points"].get(index)?)));
            field(point, "x", scale.x, scale.inverse, prior, false)?;
            field(point, "y", scale.y, scale.inverse, prior, false)?;
        }
    } else {
        if !matches!(
            kind,
            "rect" | "round-rect" | "ellipse" | "triangle" | "polygon" | "text" | "serial"
        ) {
            return Err(Error::Annotation);
        }
        field(&mut result, "x", scale.x, scale.inverse, previous, false)?;
        field(&mut result, "y", scale.y, scale.inverse, previous, false)?;
        if matches!(kind, "text" | "serial") {
            field(
                &mut result,
                "fontSize",
                scale.scalar,
                scale.inverse,
                previous,
                true,
            )?;
        } else {
            field(&mut result, "w", scale.x, scale.inverse, previous, false)?;
            field(&mut result, "h", scale.y, scale.inverse, previous, false)?;
        }
    }
    let style = result
        .get_mut("style")
        .filter(|v| v.is_object())
        .ok_or(Error::Annotation)?;
    let prior = previous.map(|(o, e)| (&o["style"], &e["style"]));
    field(style, "width", scale.scalar, scale.inverse, prior, false)?;
    field(
        style,
        "cornerRadius",
        scale.scalar,
        scale.inverse,
        prior,
        true,
    )?;
    // If original wire omitted default font, unchanged font size remains omitted; if serial-radius changes would alter the default font size,
    // retain an explicit inverse-mapped font size so local geometry edits cannot silently change text size.
    if scale.inverse && matches!(kind, "text" | "serial") {
        if let Some((original, expected)) = previous {
            if original.get("fontSize").is_none()
                && equal_optional(value.get("fontSize"), expected.get("fontSize"))
                && default_size(original) == default_size(&result)
            {
                result
                    .as_object_mut()
                    .ok_or(Error::Annotation)?
                    .remove("fontSize");
            }
        }
    }
    Ok(result)
}
pub(super) fn forward(objects: &Objects, basis: &Basis, size: Size) -> Result<Objects, Error> {
    let scale = Scale::new(basis, size, false);
    objects
        .iter()
        .map(|(id, v)| Ok((id.clone(), object(v, scale, None)?)))
        .collect()
}
pub(super) fn inverse(
    local: &Objects,
    originals: &Objects,
    expected: &Objects,
    basis: &Basis,
    size: Size,
) -> Result<Objects, Error> {
    let scale = Scale::new(basis, size, true);
    local
        .iter()
        .map(|(id, v)| {
            let prior = originals.get(id).zip(expected.get(id));
            Ok((id.clone(), object(v, scale, prior)?))
        })
        .collect()
}
