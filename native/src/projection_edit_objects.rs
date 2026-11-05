//! 协作对象在进入原生渲染前核字段/几何/文本与原Web合同；不接受脚本或样式扩展。
use super::{identifier, Error, Objects, MAX_BYTES};
use serde_json::{Map, Value};
fn number(v: &Value, min: f64, max: f64) -> bool {
    v.as_f64()
        .is_some_and(|v| v.is_finite() && (min..=max).contains(&v))
}
fn coordinate(v: &Value) -> bool {
    number(v, -1_000_000.0, 1_000_000.0)
}
fn optional(v: &Map<String, Value>, key: &str, min: f64, max: f64) -> bool {
    v.get(key).is_none_or(|v| number(v, min, max))
}
fn keys(v: &Map<String, Value>, allowed: &[&str]) -> bool {
    v.keys().all(|key| allowed.contains(&key.as_str()))
}
fn color(v: &Value) -> bool {
    let Some(v) = v.as_str().filter(|v| v.len() <= 64) else {
        return false;
    };
    // 复用有alpha的纯色解析；只放行Web v1颜色词法，不接收currentColor/资源/表达式。
    let literal = v == "transparent"
        || v.starts_with('#')
        || ((v.starts_with("rgb(") || v.starts_with("rgba("))
            && v.bytes()
                .all(|b| b.is_ascii_digit() || b"rgba().,% \t\r\n".contains(&b)));
    literal && crate::surface_color::Color::parse(v, crate::surface_color::Color::BLACK).is_some()
}
pub(super) fn annotation(id: &str, v: &Value) -> bool {
    let Some(v) = v.as_object() else {
        return false;
    };
    if !identifier(id)
        || v.get("id").and_then(Value::as_str) != Some(id)
        || !v.get("zIndex").is_some_and(coordinate)
        || serde_json::to_vec(v).map_or(true, |v| v.len() > 16 * 1024)
    {
        return false;
    }
    let Some(style) = v.get("style").and_then(Value::as_object) else {
        return false;
    };
    if !keys(
        style,
        &[
            "color",
            "width",
            "opacity",
            "fill",
            "secondaryFill",
            "cornerRadius",
            "dashPattern",
        ],
    ) || !style.get("color").is_some_and(color)
        || !style.get("width").is_some_and(|v| number(v, 0.0, 8192.0))
        || !optional(style, "opacity", 0.0, 1.0)
        || !optional(style, "cornerRadius", 0.0, 8192.0)
        || ["fill", "secondaryFill"]
            .iter()
            .any(|key| style.get(*key).is_some_and(|v| !color(v)))
        || style
            .get("dashPattern")
            .is_some_and(|v| !matches!(v.as_str(), Some("solid" | "dash-1" | "dash-2")))
    {
        return false;
    }
    let Some(kind) = v.get("type").and_then(Value::as_str) else {
        return false;
    };
    let common = ["id", "type", "zIndex", "style"];
    let extras: &[&str] = match kind {
        "rect" | "round-rect" | "ellipse" | "triangle" | "polygon" => {
            &["x", "y", "w", "h", "rotation", "sides"]
        }
        "line" | "polyline" | "arrow" | "brush" | "highlighter" => &["points"],
        "text" | "serial" => &["x", "y", "text", "fontSize", "fontFamily", "rotation"],
        _ => return false,
    };
    if v.keys()
        .any(|k| !common.contains(&k.as_str()) && !extras.contains(&k.as_str()))
    {
        return false;
    }
    if extras.contains(&"points") {
        return v.get("points").and_then(Value::as_array).is_some_and(|p| {
            !p.is_empty()
                && p.len() <= 2048
                && p.iter().all(|p| {
                    p.as_object().is_some_and(|p| {
                        keys(p, &["x", "y"])
                            && p.get("x").is_some_and(coordinate)
                            && p.get("y").is_some_and(coordinate)
                    })
                })
        });
    }
    if !v.get("x").is_some_and(coordinate)
        || !v.get("y").is_some_and(coordinate)
        || !optional(v, "rotation", -36000.0, 36000.0)
    {
        return false;
    }
    if extras.contains(&"w") {
        return v.get("w").is_some_and(|v| number(v, 0.0, 1_000_000.0))
            && v.get("h").is_some_and(|v| number(v, 0.0, 1_000_000.0))
            && v.get("sides")
                .is_none_or(|v| v.as_u64().is_some_and(|v| (3..=64).contains(&v)));
    }
    v.get("text")
        .and_then(Value::as_str)
        .is_some_and(|v| v.encode_utf16().count() <= 4096)
        && optional(v, "fontSize", 1.0, 8192.0)
        && v.get("fontFamily").is_none_or(|v| {
            v.as_str().is_some_and(|v| {
                v.encode_utf16().count() <= 128 && !v.contains([';', '{', '}', '\r', '\n'])
            })
        })
}
pub(super) fn validate(values: &Objects) -> Result<(), Error> {
    if values.len() > 256 || serde_json::to_vec(values).map_or(true, |v| v.len() > MAX_BYTES) {
        return Err(Error::Limit);
    }
    if values.iter().any(|(id, v)| !annotation(id, v)) {
        return Err(Error::Annotation);
    }
    Ok(())
}
