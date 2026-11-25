//! 原生 Surface 长度白名单与布局求值；不执行 CSS，auto/fr 的上下文由布局 owner 提供。
use serde_json::Value;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    Px,
    Percent,
    Rem,
    Em,
    Vw,
    Vh,
    Cqw,
    Cqh,
    Fr,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Auto,
    Scalar(f32, Unit),
    ContainerMin(f32, Unit, f32, Unit),
    BoundedPosition {
        percent: f32,
        offset: f32,
        bound_percent: f32,
        bound_px: f32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub axis: f32,
    pub viewport: [f32; 2],
    pub container: [f32; 2],
    pub root_font: f32,
    pub font: f32,
    pub fraction: Option<f32>,
}
fn decimal(value: &str, maximum: f32) -> Option<f32> {
    let (whole, tail) = value
        .split_once('.')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || tail.is_some_and(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let number: f32 = value.parse().ok()?;
    (number.is_finite() && number <= maximum).then_some(number)
}
fn scalar(value: &str) -> Option<(f32, Unit)> {
    for (suffix, unit, max) in [
        ("px", Unit::Px, 8192.),
        ("%", Unit::Percent, 1000.),
        ("rem", Unit::Rem, 1000.),
        ("em", Unit::Em, 1000.),
        ("vw", Unit::Vw, 1000.),
        ("vh", Unit::Vh, 1000.),
        ("cqw", Unit::Cqw, 1000.),
        ("cqh", Unit::Cqh, 1000.),
        ("fr", Unit::Fr, 1000.),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            return decimal(number, max).map(|n| (n, unit));
        }
    }
    None
}
impl Length {
    pub fn parse(value: &Value) -> Option<Self> {
        if let Some(number) = value.as_f64() {
            return (number.is_finite() && (0. ..=8192.).contains(&number))
                .then_some(Self::Scalar(number as f32, Unit::Px));
        }
        let text = value.as_str()?.trim();
        if text.len() > 256 {
            return None;
        }
        if text == "auto" {
            return Some(Self::Auto);
        }
        if text == "0" {
            return Some(Self::Scalar(0., Unit::Px));
        }
        if let Some((n, u)) = scalar(text) {
            return Some(Self::Scalar(n, u));
        }
        // 只接受协议已有的固定 min/calc 结构，没有通用表达式求值器。
        let text: String = text
            .chars()
            .filter(|c| c.is_ascii_whitespace().not())
            .collect();
        let inner = text.strip_prefix("min(")?.strip_suffix(')')?;
        if let Some((a, b)) = inner.split_once(',') {
            if let (Some((x, u)), Some((y, v))) = (scalar(a), scalar(b)) {
                if matches!(u, Unit::Cqw | Unit::Cqh) && matches!(v, Unit::Cqw | Unit::Cqh) {
                    return Some(Self::ContainerMin(x, u, y, v));
                }
            }
        }
        let (position, bound) = inner.split_once(",calc(100%-min(")?;
        let (percent, offset) = if let Some(v) = position
            .strip_prefix("calc(")
            .and_then(|v| v.strip_suffix(')'))
        {
            let (a, b) = v.split_once("%+")?;
            (decimal(a, 100.)?, decimal(b.strip_suffix("px")?, 8192.)?)
        } else {
            (decimal(position.strip_suffix('%')?, 100.)?, 0.)
        };
        let (a, b) = bound.strip_suffix("))")?.split_once("%,")?;
        Some(Self::BoundedPosition {
            percent,
            offset,
            bound_percent: decimal(a, 100.)?,
            bound_px: decimal(b.strip_suffix("px")?, 8192.)?,
        })
    }
    pub fn resolve(self, context: Context) -> Option<f32> {
        let values = [
            context.axis,
            context.viewport[0],
            context.viewport[1],
            context.container[0],
            context.container[1],
            context.root_font,
            context.font,
            context.fraction.unwrap_or(0.),
        ];
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
        {
            return None;
        }
        let scalar = |n: f32, u: Unit| -> Option<f32> {
            Some(
                n * match u {
                    Unit::Px => 1.,
                    Unit::Percent => context.axis / 100.,
                    Unit::Rem => context.root_font,
                    Unit::Em => context.font,
                    Unit::Vw => context.viewport[0] / 100.,
                    Unit::Vh => context.viewport[1] / 100.,
                    Unit::Cqw => context.container[0] / 100.,
                    Unit::Cqh => context.container[1] / 100.,
                    Unit::Fr => context.fraction?,
                },
            )
        };
        let result = match self {
            Self::Auto => return None,
            Self::Scalar(n, u) => scalar(n, u)?,
            Self::ContainerMin(a, u, b, v) => scalar(a, u)?.min(scalar(b, v)?),
            Self::BoundedPosition {
                percent,
                offset,
                bound_percent,
                bound_px,
            } => (context.axis * percent / 100. + offset)
                .min(context.axis - (context.axis * bound_percent / 100.).min(bound_px)),
        };
        (result.is_finite() && result >= 0.).then_some(result)
    }
}
use std::ops::Not;
#[cfg(test)]
#[path = "surface_length_tests.rs"]
mod tests;
