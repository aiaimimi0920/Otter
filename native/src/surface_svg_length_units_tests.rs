//! 长度单位等价关系经过生产解析、字体代理、布局及 child 绘制，不使用替身。
use super::{decode, svg};

fn assert_equivalent(wrap: impl Fn(&str) -> String, length: &str, user_units: &str) {
    let actual = decode(&wrap(length)).unwrap();
    let expected = decode(&wrap(user_units)).unwrap();
    assert_eq!(
        (actual.width, actual.height),
        (expected.width, expected.height)
    );
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels, "{length} != {user_units}");
}

#[test]
fn surface_svg_length_absolute_units_match_user_units() {
    for adjust in ["spacing", "spacingAndGlyphs"] {
        let wrap = |length: &str| {
            svg("width='200' height='80'", &format!(
                "<text x='20' y='40' font-family='Consolas' font-size='16' textLength='{length}' lengthAdjust='{adjust}'>FH</text>"
            ))
        };
        for length in ["96px", "1in", "72pt", "6pc", "25.4mm", "2.54cm"] {
            assert_equivalent(wrap, length, "96");
        }
    }
}

#[test]
fn surface_svg_length_font_relative_units_follow_inherited_size() {
    for adjust in ["spacing", "spacingAndGlyphs"] {
        let wrap = |length: &str| {
            svg("width='200' height='80'", &format!(
                "<g font-family='Consolas' font-size='24'><text x='20' y='40' textLength='{length}' lengthAdjust='{adjust}'>FH</text></g>"
            ))
        };
        assert_equivalent(wrap, "2em", "48");
    }
}

#[test]
fn surface_svg_length_percent_uses_current_viewbox() {
    for nested in [false, true] {
        for adjust in ["spacing", "spacingAndGlyphs"] {
            let wrap = |length: &str| {
                let text = format!(
                    "<text x='5' y='25' font-family='Consolas' font-size='16' textLength='{length}' lengthAdjust='{adjust}'>FH</text>"
                );
                let content = if nested {
                    format!("<svg x='10' y='10' width='80' height='80' viewBox='0 0 40 40'>{text}</svg>")
                } else {
                    text
                };
                svg("width='200' height='200' viewBox='0 0 100 100'", &content)
            };
            assert_equivalent(wrap, "50%", if nested { "20" } else { "50" });
        }
    }
}
