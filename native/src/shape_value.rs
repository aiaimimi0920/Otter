//! 与主干形状设置相同的有界文本输入；解析失败不得覆盖当前配置。
pub fn ratio(text: &str) -> Option<f32> {
    if text.encode_utf16().count() > 32 {
        return None;
    }
    let mut parts = text.split([':', '：']);
    let width = decimal(parts.next()?.trim())?;
    let height = decimal(parts.next()?.trim())?;
    if parts.next().is_some()
        || !(0.01..=10000.0).contains(&width)
        || !(0.01..=10000.0).contains(&height)
    {
        return None;
    }
    Some(width / height)
}
fn decimal(text: &str) -> Option<f32> {
    let mut parts = text.split('.');
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(parts.next()?) {
        return None;
    }
    if let Some(fraction) = parts.next() {
        if !digits(fraction) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    text.parse().ok()
}
pub fn step(text: &str) -> Option<u16> {
    let text = text.trim();
    if text.is_empty() || text.len() > 3 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ratio_matches_baseline_decimal_grammar_and_bounds() {
        assert_eq!(ratio(" 2.5 ： 1.25 "), Some(2.0));
        assert_eq!(ratio("0.01:10000"), Some(0.000001));
        for text in [
            "0:1", "-1:1", "1e2:1", ".5:1", "1.:1", "1:1:1", "NaN:1", "10001:1", "1:0.001", "",
        ] {
            assert_eq!(ratio(text), None, "{text}");
        }
        assert_eq!(ratio(&format!("{}1:1", " ".repeat(32))), None);
    }
    #[test]
    fn step_rejects_invalid_without_rounding() {
        assert_eq!(step("0"), Some(0));
        assert_eq!(step(" 999 "), Some(999));
        for text in ["-1", "1000", "1.5", "1e2", "", "NaN"] {
            assert_eq!(step(text), None);
        }
    }
}
