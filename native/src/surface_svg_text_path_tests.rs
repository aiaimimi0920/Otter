//! Chrome 已验证的路径文字等价关系；实际字体与栅格化仍由受限 Job child 完成。
use super::*;

fn assert_same_pixels(actual: &Image, expected: &Image, context: &str) {
    assert_eq!(
        (actual.width, actual.height),
        (expected.width, expected.height)
    );
    let difference = actual
        .pixels
        .iter()
        .zip(expected.pixels.iter())
        .position(|(a, b)| a != b);
    assert_eq!(difference, None, "{context}: first differing byte");
}

#[test]
fn surface_svg_text_paths_anchor_reversed_offsets_without_reordering_glyphs() {
    let style = "font-family='Consolas' font-size='20' fill='red'";
    for scope in ["text", "tspan"] {
        for (anchor, first_end, second_start) in
            [("start", 150, 110), ("middle", 130, 90), ("end", 110, 70)]
        {
            for path in ["M10 40H220", "M10 40H10H130H220"] {
                let parent_dx = if scope == "text" { " dx='0 -40'" } else { "" };
                let content = if scope == "tspan" {
                    "<tspan dx='0 -40'>FH</tspan>"
                } else {
                    "FH"
                };
                let source = format!("<defs><path id='p' d='{path}'/></defs><text {style} text-anchor='{anchor}'{parent_dx}><textPath href='#p' startOffset='100'>{content}</textPath></text>");
                let expected = format!("<text {style} x='{first_end}' y='40' text-anchor='end'>F</text><text {style} x='{second_start}' y='40'>H</text>");
                let actual = decode(&svg("width='240' height='80'", &source)).unwrap();
                let expected = decode(&svg("width='240' height='80'", &expected)).unwrap();
                assert!(expected.pixels.chunks_exact(4).any(|p| p[3] != 0));
                assert_same_pixels(&actual, &expected, &source);
            }
        }
    }
}

#[test]
fn surface_svg_text_paths_ignore_dx_on_textpath_itself() {
    let source = "<defs><path id='p' d='M10 40H220'/></defs><text font-family='Consolas' font-size='20'><textPath href='#p'>FH</textPath></text>";
    let expected = decode(&svg("width='240' height='80'", source)).unwrap();
    for dx in ["0 -100", "0 40", "40 0", "-100 100"] {
        let content = source.replace("href='#p'", &format!("href='#p' dx='{dx}'"));
        let actual = decode(&svg("width='240' height='80'", &content)).unwrap();
        assert_same_pixels(&actual, &expected, &content);
    }
}

#[test]
fn surface_svg_text_paths_match_straight_text_offsets_and_anchors() {
    for (offset, x, anchor) in [
        ("0", 10, "start"),
        ("40", 50, "start"),
        ("25%", 50, "start"),
        ("50%", 90, "middle"),
        ("50%", 90, "end"),
    ] {
        let style =
            format!("font-family='Consolas' font-size='20' text-anchor='{anchor}' fill='red'");
        let actual = decode(&svg("width='200' height='60'", &format!(
            "<defs><path id='p' d='M10 30H170'/></defs><text {style}><textPath href='#p' startOffset='{offset}'>H</textPath></text>"
        ))).unwrap();
        let expected = decode(&svg(
            "width='200' height='60'",
            &format!("<text x='{x}' y='30' {style}>H</text>"),
        ))
        .unwrap();
        assert!(expected.pixels.chunks_exact(4).any(|p| p[3] != 0));
        assert_same_pixels(
            &actual,
            &expected,
            &format!("offset={offset}, anchor={anchor}"),
        );
    }
}

#[test]
fn surface_svg_text_paths_preserve_vertical_rotation_and_curved_translation() {
    let style = "font-family='Consolas' font-size='20' fill='red'";
    for (actual, expected) in [
        (format!("<defs><path id='p' d='M40 10V55'/></defs><text {style}><textPath href='#p'>H</textPath></text>"),
         format!("<text transform='translate(40 10) rotate(90)' {style}>H</text>")),
        (format!("<defs><path id='p' d='M10 40Q90 -10 170 40' transform='translate(10 5)'/></defs><text {style}><textPath href='#p'>Otter</textPath></text>"),
         format!("<defs><path id='p' d='M10 40Q90 -10 170 40'/></defs><text transform='translate(10 5)' {style}><textPath href='#p'>Otter</textPath></text>")),
    ] {
        let source = svg("width='200' height='60'", &actual);
        let actual = decode(&source).unwrap();
        let expected = decode(&svg("width='200' height='60'", &expected)).unwrap();
        assert!(expected.pixels.chunks_exact(4).any(|p| p[3] != 0));
        assert_same_pixels(&actual, &expected, &source);
    }
}

#[test]
fn surface_svg_text_paths_missing_or_past_end_are_not_drawn() {
    for (reference, offset) in [("absent", "0"), ("p", "200%")] {
        let image = decode(&svg("width='200' height='60'", &format!(
            "<defs><path id='p' d='M10 30H170'/></defs><text font-family='Consolas' font-size='20'><textPath href='#{reference}' startOffset='{offset}'>H</textPath></text>"
        ))).unwrap();
        assert!(image.pixels.iter().all(|v| *v == 0));
    }
}

#[test]
fn surface_svg_text_paths_reject_external_ambiguous_and_recursive_references() {
    for content in [
        "<text><textPath href='https://example.invalid/a.svg#p'>H</textPath></text>",
        "<text><textPath href='file:///C:/path.svg#p'>H</textPath></text>",
        "<path id='p'/><path id='p'/><text><textPath href='#p'>H</textPath></text>",
        "<path id='p'><text><textPath href='#p'>H</textPath></text></path>",
        "<text><textPath href='#p' startOffset='1e999'>H</textPath></text>",
    ] {
        assert!(
            decode(&svg("width='200' height='60'", content)).is_err(),
            "{content}"
        );
    }
}
