//! 首批静态矢量准入。未实现内容整体失败，不能交给 usvg 静默忽略。
use super::invalid;
use resvg::usvg::roxmltree::{Document, ParsingOptions};
use std::io;
const SVG: &str = "http://www.w3.org/2000/svg";
const XLINK: &str = "http://www.w3.org/1999/xlink";
pub(super) fn parse(source: &str) -> io::Result<Document<'_>> {
    Document::parse_with_options(
        source,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            entity_resolver: None,
        },
    )
    .map_err(|_| invalid())
}
pub(super) fn validate(doc: &Document<'_>) -> io::Result<()> {
    let root = doc.root_element();
    if root.tag_name().namespace() != Some(SVG) || root.tag_name().name() != "svg" {
        return Err(invalid());
    }
    for node in doc.descendants() {
        if node.is_pi() {
            return Err(invalid());
        }
        if !node.is_element() {
            continue;
        }
        if node.tag_name().namespace() != Some(SVG)
            || !matches!(
                node.tag_name().name(),
                "svg"
                    | "g"
                    | "defs"
                    | "path"
                    | "rect"
                    | "circle"
                    | "ellipse"
                    | "line"
                    | "polyline"
                    | "polygon"
                    | "linearGradient"
                    | "radialGradient"
                    | "stop"
                    | "clipPath"
                    | "mask"
                    | "pattern"
                    | "marker"
                    | "symbol"
                    | "use"
                    | "title"
                    | "desc"
                    | "text"
                    | "tspan"
                    | "textPath"
            )
        {
            return Err(invalid());
        }
        // 嵌套 viewport 由 renderer 的 transform/clip 管理；只有根参与自然尺寸规范化。
        for attr in node.attributes() {
            if attr.namespace().is_some()
                && !(attr.namespace() == Some(XLINK) && attr.name() == "href")
            {
                return Err(invalid());
            }
            let value = attr.value();
            match attr.name() {
                // Element-owned constraints are solved inside the bounded worker.
                // Match the renderer's scalar conversion so overflow/underflow cannot
                // silently turn an explicit constraint into an ignored target.
                "textLength" if matches!(node.tag_name().name(), "text" | "tspan" | "textPath") => {
                    let length = value.parse::<svgtypes::Length>().map_err(|_| invalid())?;
                    let scalar = length.number as f32;
                    if !scalar.is_finite() || (scalar == 0.0 && length.number != 0.0) {
                        return Err(invalid());
                    }
                }
                "lengthAdjust"
                    if matches!(node.tag_name().name(), "text" | "tspan" | "textPath")
                        && matches!(value, "spacing" | "spacingAndGlyphs") => {}
                "direction" if matches!(value, "ltr" | "rtl") => {}
                "paint-order" if valid_paint_order(value) => {}
                "overflow"
                    if matches!(
                        node.tag_name().name(),
                        "svg" | "symbol" | "pattern" | "marker"
                    ) && matches!(value, "visible" | "hidden" | "scroll" | "auto") => {}
                "href" if value.starts_with('#') && value.len() > 1 => {}
                "startOffset" if node.tag_name().name() == "textPath" => {
                    let length = value.parse::<svgtypes::Length>().map_err(|_| invalid())?;
                    if !length.number.is_finite() {
                        return Err(invalid());
                    }
                }
                "fill" | "stroke" => {
                    svgtypes::Paint::from_str(value).map_err(|_| invalid())?;
                }
                "clip-path" | "mask" | "marker-start" | "marker-mid" | "marker-end"
                    if value == "none" => {}
                "clip-path" | "mask" | "marker-start" | "marker-mid" | "marker-end" => {
                    svgtypes::FuncIRI::from_str(value).map_err(|_| invalid())?;
                }
                "maskUnits" | "maskContentUnits"
                    if matches!(value, "userSpaceOnUse" | "objectBoundingBox") => {}
                "mask-type" if matches!(value, "alpha" | "luminance") => {}
                "markerUnits" if matches!(value, "userSpaceOnUse" | "strokeWidth") => {}
                "orient" if matches!(value, "auto" | "auto-start-reverse") => {}
                "orient" => {
                    value.parse::<svgtypes::Angle>().map_err(|_| invalid())?;
                }
                "markerWidth" | "markerHeight" | "refX" | "refY" => {}
                "patternUnits" | "patternContentUnits"
                    if matches!(value, "userSpaceOnUse" | "objectBoundingBox") => {}
                "patternTransform" => {
                    value
                        .parse::<svgtypes::Transform>()
                        .map_err(|_| invalid())?;
                }
                "id"
                | "version"
                | "width"
                | "height"
                | "viewBox"
                | "preserveAspectRatio"
                | "x"
                | "y"
                | "x1"
                | "y1"
                | "x2"
                | "y2"
                | "cx"
                | "cy"
                | "r"
                | "rx"
                | "ry"
                | "fx"
                | "fy"
                | "fr"
                | "d"
                | "points"
                | "transform"
                | "opacity"
                | "fill-opacity"
                | "stroke-opacity"
                | "fill-rule"
                | "clip-rule"
                | "stroke-width"
                | "stroke-linecap"
                | "stroke-linejoin"
                | "stroke-miterlimit"
                | "stroke-dasharray"
                | "stroke-dashoffset"
                | "color"
                | "stop-color"
                | "stop-opacity"
                | "offset"
                | "gradientUnits"
                | "gradientTransform"
                | "spreadMethod"
                | "clipPathUnits"
                | "display"
                | "visibility" => {}
                "font-family" | "font-size" | "font-weight" | "font-style" | "font-stretch"
                | "text-anchor" | "dx" | "dy" | "rotate" | "letter-spacing" | "word-spacing" => {}
                _ => return Err(invalid()),
            }
        }
    }
    Ok(())
}

// svgtypes intentionally falls back to normal on malformed input. Validate first
// so unsupported or duplicated tokens cannot silently change the requested order.
fn valid_paint_order(value: &str) -> bool {
    let mut tokens = value
        .split([' ', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty());
    let mut seen = 0;
    while let Some(token) = tokens.next() {
        let bit = match token {
            "normal" => return seen == 0 && tokens.next().is_none(),
            "fill" => 1,
            "stroke" => 2,
            "markers" => 4,
            _ => return false,
        };
        if seen & bit != 0 {
            return false;
        }
        seen |= bit;
    }
    seen != 0
}
