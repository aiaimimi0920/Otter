//! 对齐 NumberControl 的有限数字、clampOptional 与十位小数规范化。
pub(super) fn parse(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return Some(0.0);
    }
    let radix = if text.starts_with("0x") || text.starts_with("0X") {
        Some(16)
    } else if text.starts_with("0b") || text.starts_with("0B") {
        Some(2)
    } else if text.starts_with("0o") || text.starts_with("0O") {
        Some(8)
    } else {
        None
    };
    let number = if let Some(radix) = radix {
        if text.len() == 2 {
            return None;
        }
        let mut number = 0.0;
        for digit in text[2..].chars() {
            number = number * radix as f64 + digit.to_digit(radix)? as f64;
        }
        number
    } else {
        text.parse::<f64>().ok()?
    };
    number.is_finite().then_some(number)
}
pub(super) fn normalize(mut value: f64, min: Option<f64>, max: Option<f64>) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    if let Some(min) = min {
        value = value.max(min);
    }
    if let Some(max) = max {
        value = value.min(max);
    }
    if value.abs() >= 1e21 {
        return Some(value);
    }
    // 从 IEEE 尾数精确舍入到十位小数，避免乘浮点 1e10 带来的双重舍入。
    let bits = value.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let power = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let scaled = mantissa as u128 * 10_000_000_000u128;
    let rounded = if power >= 0 {
        scaled.checked_shl(power as u32)?
    } else if -power >= 128 {
        0
    } else {
        let shift = (-power) as u32;
        let half = 1u128 << (shift - 1);
        (scaled >> shift) + u128::from((scaled & ((1u128 << shift) - 1)) >= half)
    };
    format!(
        "{}{}.{:010}",
        if value.is_sign_negative() { "-" } else { "" },
        rounded / 10_000_000_000,
        rounded % 10_000_000_000
    )
    .parse()
    .ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn main_number_syntax_clamping_and_precision() {
        for (text, expected) in [
            ("", 0.0),
            ("+2", 2.0),
            (".5", 0.5),
            ("0x10", 16.0),
            ("0b11", 3.0),
            ("0o10", 8.0),
        ] {
            assert_eq!(parse(text), Some(expected));
        }
        for text in ["NaN", "Infinity", "1e999", "0x", "-0x1", "1_000"] {
            assert!(parse(text).is_none());
        }
        assert_eq!(normalize(12.0, Some(0.0), Some(10.0)), Some(10.0));
        assert_eq!(normalize(3.5, None, None), Some(3.5));
        assert_eq!(normalize(0.1 + 0.2, None, None), Some(0.3));
        assert_eq!(normalize(1e-100, None, None), Some(0.0));
        assert_eq!(normalize(1e30, None, None), Some(1e30));
        // 由 Node 的 Number(x.toFixed(10)) 实际核对的二进制舍入边界。
        for (input, expected) in [
            (5e-11, 1e-10),
            (1.5e-10, 1e-10),
            (2.5e-10, 3e-10),
            (-5e-11, -1e-10),
            (1.23456789005, 1.23456789),
        ] {
            assert_eq!(normalize(input, None, None), Some(expected));
        }
    }
}
