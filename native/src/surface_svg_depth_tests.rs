//! 深层 XML 与文字经过真实 worker；不放宽 Job、字体预算或属性准入。
use super::{decode, svg};

#[test]
fn surface_svg_nested_text_matches_flat_pixels_and_preserves_followup() {
    let wrap = |body: &str| {
        svg(
            "width='300' height='80'",
            &format!("<text x='20' y='40' font-family='Consolas' font-size='16'>{body}</text>"),
        )
    };
    let expected = decode(&wrap("FH")).unwrap();
    for depth in [32, 128, 512] {
        let nested = format!("{}FH{}", "<tspan>".repeat(depth), "</tspan>".repeat(depth));
        let actual = decode(&wrap(&nested)).unwrap();
        assert_eq!(actual.pixels, expected.pixels, "depth={depth}");
        assert_eq!(decode(&wrap("FH")).unwrap().pixels, expected.pixels);
    }
}

#[test]
fn surface_svg_deep_malformed_xml_is_rejected_before_next_valid_request() {
    for ending in ["", "</wrong>", "</tspan>"] {
        let body = format!("<text>{}FH{ending}</text>", "<tspan>".repeat(512));
        assert!(decode(&svg("width='10' height='10'", &body)).is_err());
        let next = decode(&svg(
            "width='1' height='1'",
            "<rect width='1' height='1' fill='red'/>",
        ))
        .unwrap();
        assert_eq!(&*next.pixels, &[0, 0, 255, 255]);
    }
}
