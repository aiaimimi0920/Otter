//! 浏览器确认的重叠 glyph 绘制顺序；通过真实受限 SVG child 验证。
use super::*;

#[test]
fn surface_svg_overlapping_glyphs_preserve_individual_paint() {
    for attrs in [
        "fill='red'",
        "fill='red' fill-opacity='.5'",
        "fill='none' stroke='blue' stroke-width='2'",
        "fill='red' stroke='blue' stroke-width='2'",
        "fill='red' fill-rule='evenodd'",
    ] {
        let style = format!("font-family='Consolas' font-size='20' {attrs}");
        let content = format!("<text x='20' y='40' {style} dx='0 -7'>FH</text>");
        // 隐形前缀保留真实字体 advance，不硬编码浏览器/原生的字体度量舍入。
        let expected = format!("<text x='20' y='40' {style}>F</text><text x='13' y='40' {style}><tspan fill='none' stroke='none'>F</tspan>H</text>");
        for opacity in ["1", ".5"] {
            let wrap = |text| {
                svg(
                    "width='120' height='70'",
                    &format!("<g opacity='{opacity}'>{text}</g>"),
                )
            };
            let actual = decode(&wrap(content.as_str())).unwrap();
            let expected = decode(&wrap(expected.as_str())).unwrap();
            assert!(expected.pixels.chunks_exact(4).any(|p| p[3] != 0));
            let difference = actual
                .pixels
                .iter()
                .zip(expected.pixels.iter())
                .position(|(a, b)| a != b);
            assert_eq!(difference, None, "{attrs}; group opacity={opacity}");
        }
    }
}

#[test]
fn surface_svg_outline_runs_remain_within_child_budget() {
    let style = "font-family='Consolas' font-size='20' fill='red'";
    let render = |count| {
        decode(&svg(
            "width='120' height='70'",
            &format!("<text x='20' y='40' {style}>{}</text>", "H".repeat(count)),
        ))
        .unwrap()
    };
    let short = render(20);
    let long = render(4096);
    assert!(long.pixels.chunks_exact(4).any(|p| p[3] != 0));
    assert_eq!(short.pixels.as_ref(), long.pixels.as_ref());
}
