//! 浏览器像素基线与真实 SVG worker 回归；不绕过生产准入与字体代理。
use super::{decode, svg};

#[test]
fn surface_svg_length_rejects_invalid_values_and_wrong_element() {
    for body in [
        "<text textLength='NaN'>FH</text>",
        "<text textLength='1e999'>FH</text>",
        "<text textLength='1e100'>FH</text>",
        "<text textLength='-1e100'>FH</text>",
        "<text textLength='3.5e38'>FH</text>",
        "<text textLength='1e100px'>FH</text>",
        "<text textLength='1e-100'>FH</text>",
        "<text textLength='-1e-100'>FH</text>",
        "<text textLength='broken'>FH</text>",
        "<text lengthAdjust='unknown'>FH</text>",
        "<text direction='auto'>FH</text>",
        "<rect textLength='80' width='10' height='10'/>",
        "<rect lengthAdjust='spacing' width='10' height='10'/>",
        "<text style='direction:rtl'>FH</text>",
    ] {
        assert!(
            decode(&svg("width='200' height='80'", body)).is_err(),
            "accepted {body}"
        );
    }
}

#[test]
fn surface_svg_length_preserves_nonpositive_length_semantics() {
    let wrap = |attribute: &str| {
        svg(
            "width='200' height='80'",
            &format!(
                "<text x='20' y='40' font-family='Consolas' font-size='16' {attribute}>FH</text>"
            ),
        )
    };
    let expected = decode(&wrap("")).unwrap();
    for attribute in ["textLength='0'", "textLength='-0'", "textLength='-10'"] {
        let actual = decode(&wrap(attribute)).unwrap();
        assert_eq!(actual.pixels, expected.pixels);
    }
}

#[test]
fn surface_svg_length_large_sibling_workload_and_followup() {
    // Exercise the real font broker and bounded worker, not an in-process renderer.
    for count in [32, 512, 4096] {
        let body = format!(
            "<text x='20' y='40' font-family='Consolas' font-size='16'>{}</text>",
            "<tspan textLength='20'>FH</tspan>".repeat(count)
        );
        let source = svg("width='300' height='80'", &body);
        let start = std::time::Instant::now();
        let image = decode(&source).unwrap();
        assert_eq!((image.width, image.height), (300, 80));
        assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
        eprintln!(
            "native_siblings={count} glyphs={} input_bytes={} elapsed_ms={}",
            count * 2,
            source.len(),
            start.elapsed().as_millis()
        );
        // A subsequent ordinary request must not inherit worker state/resources.
        let next = decode(&svg(
            "width='1' height='1'",
            "<rect width='1' height='1' fill='red'/>",
        ))
        .unwrap();
        assert_eq!(&*next.pixels, &[0, 0, 255, 255]);
    }
}

#[test]
fn surface_svg_length_nested_length_workload_and_followup() {
    for depth in [32, 128, 512] {
        // Equal spacing targets are intentional: deep ancestry must not make
        // the two visible glyphs disappear or change their resolved positions.
        let text = format!(
            "{}FH{}",
            "<tspan textLength='80'>".repeat(depth),
            "</tspan>".repeat(depth)
        );
        let wrap = |body: &str| {
            svg(
                "width='300' height='80'",
                &format!("<text x='20' y='40' font-family='Consolas' font-size='16'>{body}</text>"),
            )
        };
        let expected = decode(&wrap("<tspan textLength='80'>FH</tspan>")).unwrap();
        let source = wrap(&text);
        let start = std::time::Instant::now();
        let actual = decode(&source).unwrap();
        assert_eq!(actual.pixels, expected.pixels, "depth={depth}");
        eprintln!(
            "native_depth={depth} input_bytes={} elapsed_ms={}",
            source.len(),
            start.elapsed().as_millis()
        );
        let followup = decode(&wrap("<tspan textLength='80'>FH</tspan>")).unwrap();
        assert_eq!(followup.pixels, expected.pixels);
    }
}
