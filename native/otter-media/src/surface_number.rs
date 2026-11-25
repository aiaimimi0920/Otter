//! Number 的字符串/数值边界及有界步进；不把手工越界输入钳制，也不拥有动作权限。
use crate::surface_scene::Node;

/// 编辑态允许缺失数字，保证删除/插入可以修复未完成的尾数或指数；不是提交格式校验。
pub fn editable(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
    }
    at == bytes.len()
}
#[test]
fn surface_number_editable_grammar_accepts_intermediates_not_arbitrary_text() {
    for value in [
        "", "+", "-", ".", "-.", "1.", "e", "e+", "1e-", "-.5E+3", "1e400",
    ] {
        assert!(editable(value), "{value}");
    }
    for value in [
        "a", " 1", "1 ", "1,2", "１２", "1\0", "--1", "1..2", "1e2e3", "1e.3",
    ] {
        assert!(!editable(value), "{value}");
    }
}

fn on_grid(value: f64) -> bool {
    value.is_finite()
        && (value - value.round()).abs() <= (8. * f64::EPSILON * value.abs()).min(1e-7)
}

pub fn parse(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    let start = at;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    let integer = at > start;
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return None;
        }
    } else if !integer {
        return None;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    if at != bytes.len() {
        return None;
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}
pub fn sanitized(text: &str) -> &str {
    if parse(text).is_some() {
        text
    } else {
        ""
    }
}

#[derive(Clone, Copy)]
pub struct Limits {
    min: f64,
    max: f64,
    step: f64,
    base: f64,
}
impl Limits {
    pub fn from_node(node: &Node) -> Self {
        let number = |key, fallback| {
            node.props()
                .get(key)
                .and_then(|v| v.as_f64())
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        // 旧 Surface numberProp 的 min/max 缺省均为 0，不改写成无上下界。
        let min = number("min", 0.);
        let max = number("max", 0.);
        let step = number("step", 1.);
        Self {
            min,
            max,
            base: min,
            step: if step > 0. { step } else { 1. },
        }
    }
    pub fn step(&self, text: &str, up: bool) -> Option<String> {
        if self.min > self.max {
            return None;
        }
        let old = parse(text).unwrap_or(0.);
        let quotient = (old - self.base) / self.step;
        // 超出整数精度或溢出时不猜测下一格，不创建无界迭代。
        if !quotient.is_finite() || quotient.abs() >= (1_u64 << 52) as f64 {
            return None;
        }
        let nearest = quotient.round();
        let index = if on_grid(quotient) {
            nearest + if up { 1. } else { -1. }
        } else if up {
            quotient.ceil()
        } else {
            quotient.floor()
        };
        let top = (self.max - self.base) / self.step;
        let rounded = top.round();
        let ceiling = if on_grid(top) { rounded } else { top.floor() };
        if !ceiling.is_finite() || ceiling < 0. {
            return None;
        }
        let mut next = self.base + index.clamp(0., ceiling) * self.step;
        if on_grid(top) && index.clamp(0., ceiling) == rounded {
            next = self.max;
        }
        if !next.is_finite()
            || next < self.min
            || next > self.max
            || (up && next < old)
            || (!up && next > old)
            || (next == old && parse(text).is_some())
        {
            return None;
        }
        Some(next.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_number_step_tolerance_never_snaps_to_large_off_grid_max() {
        let limits = Limits {
            min: 0.,
            max: 1e15 + 0.25,
            step: 1.,
            base: 0.,
        };
        assert_eq!(
            limits.step("999999999999999", true).as_deref(),
            Some("1000000000000000")
        );
        let small = Limits {
            min: 0.,
            max: 1e-300,
            step: 1.,
            base: 0.,
        };
        assert_eq!(small.step("0", true), None);
        let decimal = Limits {
            min: 0.,
            max: 0.3,
            step: 0.1,
            base: 0.,
        };
        assert_eq!(decimal.step("0.2", true).as_deref(), Some("0.3"));
    }
    #[test]
    fn surface_number_strict_values_preserve_strings_without_accepting_rust_extensions() {
        for text in ["0", "-0", ".5", "-.5", "1.00", "1e+2", "1E-3"] {
            assert!(parse(text).is_some(), "{text}");
            assert_eq!(sanitized(text), text);
        }
        for text in [
            "", "-", ".", "1.", "1e", "+1", " 1", "1 ", "NaN", "inf", "Infinity", "0x10", "１",
            "1e400", "1\0",
        ] {
            assert_eq!(parse(text), None, "{text}");
            assert_eq!(sanitized(text), "");
        }
    }
    #[test]
    fn surface_number_step_aligns_and_clamps_without_reversing_direction() {
        let limits = Limits {
            min: -2.,
            max: 3.,
            step: 0.5,
            base: -2.,
        };
        assert_eq!(limits.step("0.2", true).as_deref(), Some("0.5"));
        assert_eq!(limits.step("0.2", false).as_deref(), Some("0"));
        assert_eq!(limits.step("", true).as_deref(), Some("0.5"));
        assert_eq!(limits.step("3", true), None);
        assert_eq!(limits.step("8", true), None);
        assert_eq!(limits.step("8", false).as_deref(), Some("3"));
        assert_eq!(limits.step("1e300", true), None);
    }
    #[test]
    fn surface_number_mapper_keeps_zero_bounds_and_invalid_step_fallback() {
        let scene = crate::surface_scene::Scene::parse(
            br#"{"id":"n","type":"number","props":{"value":"12","step":"any"}}"#,
        )
        .unwrap();
        let limits = Limits::from_node(scene.root());
        assert_eq!(
            (limits.min, limits.max, limits.step, limits.base),
            (0., 0., 1., 0.)
        );
        assert_eq!(limits.step("0", true), None);
        assert_eq!(limits.step("", true).as_deref(), Some("0"));
    }
}
