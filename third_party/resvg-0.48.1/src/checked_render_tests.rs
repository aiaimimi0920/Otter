//! Filter errors must survive subroot boundaries and not leak into later calls.
fn document(body: &str, frequency: &str) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='16' height='16'><defs><filter id='bad' x='0' y='0' width='100%' height='100%'><feTurbulence baseFrequency='{frequency}'/></filter></defs>{body}</svg>"
    )
}
fn checked(source: &str) -> Result<(), crate::RenderError> {
    let tree = crate::usvg::Tree::from_str(source, &crate::usvg::Options::default()).unwrap();
    let mut pixels = crate::tiny_skia::Pixmap::new(16, 16).unwrap();
    crate::render_checked(
        &tree,
        crate::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    )
}
#[test]
fn nested_failure_and_next_request() {
    let cases = [
        (
            "direct",
            "<rect width='16' height='16' filter='url(#bad)'/>",
        ),
        (
            "pattern",
            "<defs><pattern id='p' width='16' height='16' patternUnits='userSpaceOnUse'><rect width='16' height='16' filter='url(#bad)'/></pattern></defs><rect width='16' height='16' fill='url(#p)'/>",
        ),
        (
            "mask",
            "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='16' height='16'><rect width='16' height='16' filter='url(#bad)'/></mask></defs><rect width='16' height='16' mask='url(#m)'/>",
        ),
        (
            "feImage",
            "<defs><rect id='r' width='16' height='16' filter='url(#bad)'/><filter id='outer' x='0' y='0' width='100%' height='100%'><feImage xlink:href='#r'/></filter></defs><rect width='16' height='16' filter='url(#outer)'/>",
        ),
    ];
    for (name, body) in cases {
        assert!(
            checked(&document(body, "1e9 0")).is_err(),
            "lost error in {name}"
        );
        assert!(
            checked(&document(body, ".1 .2")).is_ok(),
            "stale error in {name}"
        );
        println!("nested={name} rejected=true next_normal=true");
    }
    // Percent-encoded SVG exercises ImageKind::SVG, not feImage's reference root.
    let inner = document("<rect width='16' height='16' filter='url(#bad)'/>", "1e9 0");
    let encoded: String = inner.bytes().map(|b| format!("%{b:02X}")).collect();
    let image = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><image width='16' height='16' href='data:image/svg+xml,{encoded}'/></svg>"
    );
    assert!(checked(&image).is_err(), "lost error in SVG image");
    assert!(checked(&document("<rect width='16' height='16'/>", ".1 .2")).is_ok());
}

#[test]
fn transformed_lattice_and_stitch_overflow_are_explicit_errors() {
    let cases = [
        ("1e9 0", 1, "noStitch", "", "16"),
        ("0 1e9", 1, "noStitch", "", "16"),
        ("1e8 .1", 1, "stitch", "", "16"),
        ("1e6 0", 9, "stitch", "", "16"),
        (
            ".2 .2",
            9,
            "noStitch",
            "translate(.25 .25) scale(2e-7)",
            "8e7",
        ),
    ];
    for (frequency, octaves, stitch, transform, extent) in cases {
        let source = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><defs><filter id='f' x='0' y='0' width='100%' height='100%'><feTurbulence baseFrequency='{frequency}' numOctaves='{octaves}' seed='3' stitchTiles='{stitch}'/></filter></defs><g transform='{transform}'><rect width='{extent}' height='{extent}' filter='url(#f)'/></g></svg>"
        );
        assert_eq!(checked(&source), Err(crate::RenderError::FilterFailed));
    }
}

fn pixel_hash(bytes: &[u8]) -> u64 {
    // Golden normal-render pixels recorded before the arithmetic fix.
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn ordinary_pixels_are_unchanged() {
    for (stitch, expected) in [
        ("noStitch", 8482639151651547215u64),
        ("stitch", 12828862321031759723u64),
    ] {
        let source = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><defs><filter id='f' x='0' y='0' width='100%' height='100%'><feTurbulence baseFrequency='.1 .2' numOctaves='9' seed='3' stitchTiles='{stitch}'/></filter></defs><rect width='16' height='16' filter='url(#f)'/></svg>"
        );
        let tree = crate::usvg::Tree::from_str(&source, &crate::usvg::Options::default()).unwrap();
        let mut pixels = crate::tiny_skia::Pixmap::new(16, 16).unwrap();
        crate::render_checked(
            &tree,
            crate::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        )
        .unwrap();
        assert_eq!(pixel_hash(pixels.data()), expected, "{stitch}");
    }
}
