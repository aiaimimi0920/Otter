//! 会话纯色边界：保留独立 alpha，复用有界 CSS 颜色解析，不接资源或 currentColor。
use crate::stroke_style::Color;

pub(super) fn parse(text: &str) -> Option<(Color, f32)> {
    if text.len() > 128 || !text.is_ascii() {
        return None;
    }
    let text = text.trim().to_ascii_lowercase();
    // 旧canonical颜色允许128字节。不能因Surface的64字节样式门拒绝已保存的长十进制，
    // 也不能让本地最小f32 alpha写出后无法重开；该兼容叶不改变网络/Surface预算。
    if text.len() > 64 {
        return long_literal(&text);
    }
    if text != "transparent"
        && !text.starts_with('#')
        && !text.starts_with("rgb(")
        && !text.starts_with("rgba(")
    {
        return None;
    }
    let [r, g, b, alpha] =
        crate::surface_color::Color::parse(&text, crate::surface_color::Color::BLACK)?.0;
    let byte = |v: f32| (v * 255.0).round() as u8;
    Some((Color(byte(r), byte(g), byte(b)), alpha))
}

fn long_literal(text: &str) -> Option<(Color, f32)> {
    let (name, body) = text.split_once('(')?;
    if !matches!(name, "rgb" | "rgba") {
        return None;
    }
    let mut parts = body.strip_suffix(')')?.split(',').map(str::trim);
    // 保留旧canonical的整数RGB长写法，不拓宽为命名色/表达式/资源。
    let color = Color(
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    let alpha = if name == "rgba" {
        let text = parts.next()?;
        let percent = text.ends_with('%');
        let text = text.strip_suffix('%').unwrap_or(text);
        if text.is_empty()
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || b"+-.".contains(&b))
        {
            return None;
        }
        let alpha = text.parse::<f32>().ok()?;
        if !alpha.is_finite() {
            return None;
        }
        (alpha / if percent { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    parts.next().is_none().then_some((color, alpha))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literals_keep_independent_alpha_and_reject_active_or_malformed_paints() {
        for text in [
            "#123",
            "#123f",
            "#112233",
            "#112233FF",
            "rgb(17, 34, 51)",
            "RGBA(17,34,51,1.0)",
            "rgba(17,34,51,1.0000000000000000000000)",
            "  rgb(17,34,51)  ",
        ] {
            assert_eq!(parse(text), Some((Color(17, 34, 51), 1.0)), "{text}");
        }
        for (text, color, alpha) in [
            ("#1230", Color(17, 34, 51), 0.0),
            ("#11223380", Color(17, 34, 51), 128.0 / 255.0),
            ("rgba(17,34,51,0.5)", Color(17, 34, 51), 0.5),
            ("rgb(100%,0%,0%)", Color(255, 0, 0), 1.0),
            ("rgba(100%,0%,0%,25%)", Color(255, 0, 0), 0.25),
            ("transparent", Color(0, 0, 0), 0.0),
        ] {
            assert_eq!(parse(text), Some((color, alpha)), "{text}");
        }
        for text in [
            "rgba(17,34,51,1e0)",
            "rgba(17,34,51,NaN)",
            "rgb(1,2,3,1)",
            "rgba(1,2,3)",
            "rgb(1,2,3))",
            "#色彩",
            "#12",
            "currentColor",
            "url(https://example.invalid/color)",
            "hsl(0,100%,50%)",
            "red",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
        assert!(parse(&" ".repeat(129)).is_none());
        let legacy = format!("rgba(17,34,51,1.{})", "0".repeat(80));
        assert_eq!(parse(&legacy), Some((Color(17, 34, 51), 1.0)));
        let tiny = format!("rgba(255,255,255,{})", f32::from_bits(1));
        assert_eq!(
            parse(&tiny),
            Some((Color(255, 255, 255), f32::from_bits(1)))
        );
    }
}
