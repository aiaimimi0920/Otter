//! Grid 轨道语法与资源边界；只求值受支持的长度，不执行 CSS 或任意表达式。
use super::Error;
use crate::surface_length::{Context, Length, Unit};
use serde_json::Value;
pub(super) const MAX_COLUMNS: usize = 64;
pub(super) const MAX_CELLS: usize = 16384;
#[derive(Clone, Copy, Debug)]
pub(super) enum Bound {
    Auto,
    Length(Length),
    Fraction(f32),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Track {
    pub min: Bound,
    pub max: Bound,
}
impl Track {
    pub const AUTO: Self = Self {
        min: Bound::Auto,
        max: Bound::Auto,
    };
}
fn bound(text: &str, fraction: bool) -> Result<Bound, Error> {
    if text == "auto" {
        return Ok(Bound::Auto);
    }
    // 小数先规范化；已有 Length parser 有意不接受 .75 形式。
    let normalized = if text.starts_with('.') {
        format!("0{text}")
    } else {
        text.to_owned()
    };
    match Length::parse(&Value::String(normalized)) {
        Some(Length::Scalar(v, Unit::Fr)) if fraction && v > 0. => Ok(Bound::Fraction(v)),
        Some(v @ Length::Scalar(_, Unit::Px | Unit::Percent)) => Ok(Bound::Length(v)),
        _ => Err(Error::Syntax),
    }
}
fn track(text: &str) -> Result<Track, Error> {
    if let Some(body) = text
        .strip_prefix("minmax(")
        .and_then(|v| v.strip_suffix(')'))
    {
        let (a, b) = body.split_once(',').ok_or(Error::Syntax)?;
        return Ok(Track {
            min: bound(a.trim(), false)?,
            max: bound(b.trim(), true)?,
        });
    }
    let max = bound(text, true)?;
    Ok(Track {
        min: if matches!(max, Bound::Fraction(_)) {
            Bound::Auto
        } else {
            max
        },
        max,
    })
}
pub(super) fn list(value: Option<&Value>, repeated: bool) -> Result<Vec<Track>, Error> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let source = value.as_str().ok_or(Error::Syntax)?;
    if source == "none" && repeated {
        return Ok(Vec::new());
    }
    if source.is_empty() || source.len() > 4096 || !source.is_ascii() {
        return Err(Error::Syntax);
    }
    let mut output = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (index, byte) in source.bytes().chain(std::iter::once(b' ')).enumerate() {
        match byte {
            b'(' => {
                depth += 1;
                if depth > 2 {
                    return Err(Error::Syntax);
                }
            }
            b')' => {
                if depth == 0 {
                    return Err(Error::Syntax);
                }
                depth -= 1;
            }
            v if v.is_ascii_whitespace() && depth == 0 => {
                let text = source[start..index].trim();
                start = index + 1;
                if text.is_empty() {
                    continue;
                }
                if let Some(body) = text
                    .strip_prefix("repeat(")
                    .and_then(|v| v.strip_suffix(')'))
                {
                    if !repeated {
                        return Err(Error::Syntax);
                    }
                    let (count, body) = body.split_once(',').ok_or(Error::Syntax)?;
                    let count = count.trim().parse::<usize>().map_err(|_| Error::Syntax)?;
                    let inner = list(Some(&Value::String(body.trim().to_owned())), false)?;
                    if count == 0
                        || count > MAX_COLUMNS
                        || inner
                            .len()
                            .checked_mul(count)
                            .is_none_or(|n| n + output.len() > MAX_COLUMNS)
                    {
                        return Err(Error::Limit);
                    }
                    for _ in 0..count {
                        output.extend_from_slice(&inner);
                    }
                } else {
                    output.push(track(text)?);
                }
                if output.len() > MAX_COLUMNS {
                    return Err(Error::Limit);
                }
            }
            _ => {}
        }
    }
    if depth != 0 || output.is_empty() {
        return Err(Error::Syntax);
    }
    Ok(output)
}
pub(super) fn automatic(value: Option<&Value>) -> Result<Track, Error> {
    let tracks = list(value, false)?;
    match tracks.as_slice() {
        [] => Ok(Track::AUTO),
        [one] => Ok(*one),
        _ => Err(Error::Unsupported),
    }
}
pub(super) fn gaps(
    node: &crate::surface_scene::Node,
    mut context: Context,
    inner: [f32; 2],
) -> Result<[f32; 2], Error> {
    let value = node.layout().get("gap");
    let parts = value
        .and_then(Value::as_str)
        .map(|v| v.split_ascii_whitespace().collect::<Vec<_>>());
    if parts.as_ref().is_some_and(|p| p.is_empty() || p.len() > 2) {
        return Err(Error::Syntax);
    }
    let mut result = [0.; 2];
    for (axis, key) in ["columnGap", "rowGap"].into_iter().enumerate() {
        let fallback = parts
            .as_ref()
            .map(|p| Value::String(p[if p.len() == 1 { 0 } else { 1 - axis }].to_owned()));
        let value = node.layout().get(key).or(fallback.as_ref()).or(value);
        if let Some(value) = value {
            context.axis = inner[0];
            let Some(Length::Scalar(n, Unit::Px)) = Length::parse(value) else {
                return Err(Error::Unsupported);
            };
            result[axis] = Length::Scalar(n, Unit::Px)
                .resolve(context)
                .ok_or(Error::Input)?;
        }
    }
    Ok(result)
}
