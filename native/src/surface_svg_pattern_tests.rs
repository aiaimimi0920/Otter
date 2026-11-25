//! Chrome 155 img-to-canvas baselines; actual production Job children decode, rather than rendering in the test parent.
use super::*;

#[test]
fn surface_svg_patterns_match_browser_percentages_and_template_overrides() {
    // Chrome measurements still use the outer viewport as the percentage basis; never infer and rewrite it from viewBox alone.
    assert_grid("<pattern id='p' patternUnits='userSpaceOnUse' width='4' height='2' viewBox='0 0 4 2'><rect width='50%' height='50%' fill='red'/></pattern>", ["RRRRRRRR"; 4]);
    assert_grid("<pattern id='p' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='25%' height='25%' fill='red'/></pattern>", ["RR..RR..", "........", "RR..RR..", "........"]);
    assert_grid("<pattern id='base' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='2' height='1' fill='red'/></pattern><pattern id='p' href='#base'><title>tile</title></pattern>", ["........"; 4]);
    assert_grid("<pattern id='base' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='2' height='1' fill='red'/></pattern><pattern id='p' href='#base'><desc>tile</desc></pattern>", ["........"; 4]);
    assert_grid("<pattern id='base' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='2' height='1' fill='red'/></pattern><pattern id='p' href='#base'><!-- tile --></pattern>", ["RR..RR..", "........", "RR..RR..", "........"]);
    assert_grid("<pattern id='base' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='2' height='1' fill='red'/></pattern><pattern id='p' href='#base' width='2'/>", ["RRRRRRRR", "........", "RRRRRRRR", "........"]);
}

#[test]
fn surface_svg_patterns_match_browser_overflow_tile_clipping() {
    // Chrome 155 clips this fixture to the tile even with visible; do not infer cross-tile wrapping from the specification.
    for overflow in ["hidden", "visible", "scroll", "auto"] {
        assert_grid(&format!("<pattern id='p' patternUnits='userSpaceOnUse' width='4' height='2' overflow='{overflow}'><rect x='3' width='2' height='1' fill='red'/></pattern>"), ["...R...R", "........", "...R...R", "........"]);
    }
}

#[test]
fn surface_svg_patterns_match_browser_units_viewbox_transform_and_template() {
    for (attrs, body, rows) in [
        ("patternUnits='userSpaceOnUse' width='4' height='2'", "<rect width='2' height='1' fill='red'/>", ["RR..RR..", "........", "RR..RR..", "........"]),
        ("width='.5' height='.5'", "<rect width='2' height='1' fill='red'/>", ["RR..RR..", "........", "RR..RR..", "........"]),
        ("width='.5' height='.5' patternContentUnits='objectBoundingBox'", "<rect width='.25' height='.25' fill='red'/>", ["RR..RR..", "........", "RR..RR..", "........"]),
        ("patternUnits='userSpaceOnUse' width='4' height='2' viewBox='0 0 2 2'", "<rect width='1' height='1' fill='red'/>", [".R...R..", "........", ".R...R..", "........"]),
        ("patternUnits='userSpaceOnUse' width='4' height='2' viewBox='0 0 2 2' preserveAspectRatio='none'", "<rect width='1' height='1' fill='red'/>", ["RR..RR..", "........", "RR..RR..", "........"]),
        ("patternUnits='userSpaceOnUse' width='4' height='2' patternTransform='translate(1 1)'", "<rect width='2' height='1' fill='red'/>", ["........", ".RR..RR.", "........", ".RR..RR."]),
        ("patternUnits='userSpaceOnUse' width='0' height='2'", "<rect width='2' height='1' fill='red'/>", ["........"; 4]),
    ] {
        let definition = format!("<pattern id='p' {attrs}>{body}</pattern>");
        assert_grid(&definition, rows);
    }
    assert_grid("<pattern id='base' patternUnits='userSpaceOnUse' width='4' height='2'><rect width='2' height='1' fill='red'/></pattern><pattern id='p' href='#base'/>", ["RR..RR..", "........", "RR..RR..", "........"]);
}
fn assert_grid(definition: &str, rows: [&str; 4]) {
    let source = svg(
        "width='8' height='4'",
        &format!("<defs>{definition}</defs><rect width='8' height='4' fill='url(#p)'/>"),
    );
    let image = decode(&source).unwrap();
    assert_eq!((image.width, image.height), (8, 4));
    for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
        let expected = if rows[i / 8].as_bytes()[i % 8] == b'R' {
            [0, 0, 255, 255]
        } else {
            [0; 4]
        };
        assert_eq!(pixel, expected, "{definition} at {},{}", i % 8, i / 8);
    }
}
#[test]
fn surface_svg_patterns_reject_cycles_and_external_resources() {
    for content in [
        "<pattern id='p' href='#q'/><pattern id='q' href='#p'/>",
        "<pattern id='p' width='1' height='1'><rect width='1' height='1' fill='url(#p)'/></pattern>",
        "<pattern id='p'><rect fill='url(#q)'/></pattern><pattern id='q'><rect stroke='url(#p)'/></pattern>",
        "<pattern id='p'><use href='#r'/></pattern><rect id='r' fill='url(#p)'/>",
        "<pattern id='p' href='https://example.invalid/p.svg#p'/>",
        "<pattern id='p'><image href='file:///C:/image.png'/></pattern>",
        "<pattern id='p'><rect fill='url(https://example.invalid/p.svg#p)'/></pattern>",
    ] {
        assert!(decode(&svg("width='8' height='4'", content)).is_err(), "{content}");
    }
}
