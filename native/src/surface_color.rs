//! 有界纯色解析；不执行 CSS 表达式或查找资源，主题变量由未来主题 owner 处理。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub [f32; 4]);
impl Color {
    pub const BLACK: Self = Self([0., 0., 0., 1.]);
    pub fn parse(text: &str, current: Self) -> Option<Self> {
        if text.len() > 64 || !text.is_ascii() {
            return None;
        }
        let text = text.trim().to_ascii_lowercase();
        let named = match text.as_str() {
            "currentcolor" => return Some(current),
            "transparent" => return Some(Self([0.; 4])),
            "black" => Some([0, 0, 0]),
            "white" => Some([255, 255, 255]),
            "red" => Some([255, 0, 0]),
            "green" => Some([0, 128, 0]),
            "blue" => Some([0, 0, 255]),
            "gray" | "grey" => Some([128, 128, 128]),
            "yellow" => Some([255, 255, 0]),
            "orange" => Some([255, 165, 0]),
            "purple" => Some([128, 0, 128]),
            "pink" => Some([255, 192, 203]),
            _ => None,
        };
        if let Some([r, g, b]) = named {
            return Some(Self([
                r as f32 / 255.,
                g as f32 / 255.,
                b as f32 / 255.,
                1.,
            ]));
        }
        if let Some(hex) = text.strip_prefix('#') {
            if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            let (step, scale) = match hex.len() {
                3 | 4 => (1, 15.),
                6 | 8 => (2, 255.),
                _ => return None,
            };
            let mut rgba = [1.; 4];
            for (index, component) in rgba.iter_mut().enumerate().take(hex.len() / step) {
                *component = u8::from_str_radix(&hex[index * step..(index + 1) * step], 16).ok()?
                    as f32
                    / scale;
            }
            return Some(Self(rgba));
        }
        let (name, body) = text.split_once('(')?;
        let parts: Vec<_> = body.strip_suffix(')')?.split(',').map(str::trim).collect();
        let alpha = matches!(name, "rgba" | "hsla");
        if parts.len() != if alpha { 4 } else { 3 } {
            return None;
        }
        let a = if alpha { channel(parts[3], 1.)? } else { 1. };
        match name {
            "rgb" | "rgba" => {
                // 逗号形式遵循传统 CSS 通道语法，不把混合百分比误解释为有效声明。
                if parts[..3]
                    .iter()
                    .any(|p| p.ends_with('%') != parts[0].ends_with('%'))
                {
                    return None;
                }
                Some(Self([
                    channel(parts[0], 255.)?,
                    channel(parts[1], 255.)?,
                    channel(parts[2], 255.)?,
                    a,
                ]))
            }
            "hsl" | "hsla" => {
                let h = number(parts[0])?.rem_euclid(360.) / 60.;
                let s = number(parts[1].strip_suffix('%')?)?.clamp(0., 100.) / 100.;
                let l = number(parts[2].strip_suffix('%')?)?.clamp(0., 100.) / 100.;
                let c = (1. - (2. * l - 1.).abs()) * s;
                let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
                let rgb = match h as u32 {
                    0 => [c, x, 0.],
                    1 => [x, c, 0.],
                    2 => [0., c, x],
                    3 => [0., x, c],
                    4 => [x, 0., c],
                    _ => [c, 0., x],
                };
                let m = l - c / 2.;
                Some(Self([rgb[0] + m, rgb[1] + m, rgb[2] + m, a]))
            }
            _ => None,
        }
    }
}
fn number(text: &str) -> Option<f32> {
    if !text
        .bytes()
        .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.'))
    {
        return None;
    }
    let value: f32 = text.parse().ok()?;
    value.is_finite().then_some(value)
}
fn channel(text: &str, scale: f32) -> Option<f32> {
    let value = match text.strip_suffix('%') {
        Some(text) => number(text)? / 100.,
        None => number(text)? / scale,
    };
    Some(value.clamp(0., 1.))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equivalent_literals_preserve_rgb_alpha_and_current_color() {
        for value in ["#f008", "#ff000088", "rgba(255,0,0,0.53333336)"] {
            assert_eq!(
                Color::parse(value, Color::BLACK),
                Some(Color([1., 0., 0., 136. / 255.]))
            );
        }
        for value in [
            "red",
            " RGB(255,0,0) ",
            "rgb(100%,0%,0%)",
            "hsl(360,100%,50%)",
            "hsl(-360,100%,50%)",
        ] {
            assert_eq!(
                Color::parse(value, Color::BLACK),
                Some(Color([1., 0., 0., 1.]))
            );
        }
        let current = Color([0.2, 0.3, 0.4, 0.5]);
        assert_eq!(Color::parse("currentColor", current), Some(current));
        assert_eq!(Color::parse("transparent", current), Some(Color([0.; 4])));
    }
    #[test]
    fn unsupported_or_executable_values_do_not_enter_renderer() {
        for value in [
            "var(--theme)",
            "url(x)",
            "expression(x)",
            "red!important",
            "rgb(NaN,0,0)",
            "rgba(1,2,3,1e0)",
            "#12",
            "rgb(1%,2,3)",
            "hsl(0,1,1)",
            "rgb(1 2 3)",
        ] {
            assert!(Color::parse(value, Color::BLACK).is_none(), "{value}");
        }
        assert!(Color::parse(&" ".repeat(65), Color::BLACK).is_none());
    }
}
