//! 有限 Number 转换；2/8/16 进制按有效位和 ties-to-even 舍入，避免逐位浮点累加误差。
use super::{string, Output, MAX_TEXT_BYTES};
pub(crate) fn whitespace(value: char) -> bool {
    matches!(value, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub(super) fn convert(value: Output<'_>) -> Option<f64> {
    let result = match value {
        Output::Number(value) => value,
        Output::Boolean(value) => {
            if value {
                1.0
            } else {
                0.0
            }
        }
        Output::Null => 0.0,
        Output::Image(value) => parse(value)?,
        Output::Array(_) => parse(&string::convert(value)?)?,
        Output::Json(serde_json::Value::Array(_)) => parse(&string::convert(value)?)?,
        _ => return None,
    };
    result.is_finite().then_some(result)
}
fn parse(value: &str) -> Option<f64> {
    if value.len() > MAX_TEXT_BYTES {
        return None;
    }
    let value = value.trim_matches(whitespace);
    if value.is_empty() {
        return Some(0.0);
    }
    if let Some(rest) = value.strip_prefix('0') {
        let radix = match rest.as_bytes().first() {
            Some(b'x' | b'X') => Some(4),
            Some(b'o' | b'O') => Some(3),
            Some(b'b' | b'B') => Some(1),
            _ => None,
        };
        if let Some(bits) = radix {
            return radix_number(&rest[1..], bits);
        }
    }
    value.parse::<f64>().ok()
}
fn radix_number(value: &str, bits: usize) -> Option<f64> {
    if value.is_empty() {
        return None;
    }
    let (mut count, mut significand, mut guard, mut sticky) = (0usize, 0u64, false, false);
    for character in value.chars() {
        let digit = character.to_digit(1 << bits)?;
        for shift in (0..bits).rev() {
            let bit = (digit >> shift) & 1 != 0;
            if count == 0 && !bit {
                continue;
            }
            count += 1;
            if count <= 53 {
                significand = (significand << 1) | u64::from(bit);
            } else if count == 54 {
                guard = bit;
            } else {
                sticky |= bit;
            }
        }
    }
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    if count > 1077 {
        return None;
    }
    Some((significand as f64) * 2.0f64.powi(count.saturating_sub(53) as i32))
}
