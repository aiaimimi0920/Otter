//! Chrome 155 img→canvas 的 mask 基线；经真实 Job child 检查预乘像素。
use super::*;
#[test]
fn surface_svg_masks_match_browser_alpha_luminance_and_regions() {
    let bands = "<rect width='1' height='1' fill='white'/><rect x='1' width='1' height='1' fill='red'/><rect x='2' width='1' height='1' fill='lime'/><rect x='3' width='1' height='1' fill='blue'/>";
    for (defs, alpha) in [
        (format!("<mask id='m' mask-type='luminance'>{bands}</mask>"), [255,54,182,18]),
        (format!("<mask id='m' mask-type='alpha'>{bands}</mask>"), [255;4]),
        ("<mask id='m'><rect width='4' height='1' fill='white' opacity='0.5'/></mask>".into(), [128;4]),
        ("<mask id='m' maskContentUnits='objectBoundingBox'><rect width='0.5' height='1' fill='white'/></mask>".into(), [255,255,0,0]),
        ("<mask id='m' maskUnits='userSpaceOnUse' x='1' y='0' width='2' height='1'><rect width='4' height='1' fill='white'/></mask>".into(), [0,255,255,0]),
        ("<mask id='m'/>".into(), [0;4]),
        ("<mask id='m' width='0'><rect width='4' height='1' fill='white'/></mask>".into(), [0;4]),
        ("<mask id='n'><rect width='2' height='1' fill='white'/></mask><mask id='m' mask='url(#n)'><rect width='4' height='1' fill='white'/></mask>".into(), [255;4]),
    ] {
        let source = svg("width='4' height='1'", &format!("<defs>{defs}</defs><rect width='4' height='1' fill='red' mask='url(#m)'/>"));
        let image = decode(&source).unwrap();
        assert_eq!([image.width,image.height], [4,1]);
        for (pixel,a) in image.pixels.chunks_exact(4).zip(alpha) {
            if defs.contains("mask-type='luminance'") {
                // 已观察到 tiny-skia 与 Chrome 的亮度到 u8 舍入相差 1；不能宣称逐字节一致。
                assert_eq!(&pixel[..2], &[0,0], "{defs}");
                assert_eq!(pixel[2],pixel[3], "{defs}");
                assert!(pixel[3].abs_diff(a) <= 1, "{defs}: {pixel:?} expected alpha {a}");
            } else {
                assert_eq!(pixel, &[0,0,a,a], "{defs}");
            }
        }
    }
}
#[test]
fn surface_svg_masks_cannot_introduce_external_resources() {
    for target in [
        "https://example.invalid/m.svg#m",
        "file:///C:/mask.svg#m",
        "data:image/svg+xml,abc",
    ] {
        let source = svg(
            "width='4' height='1'",
            &format!("<rect width='4' height='1' mask='url({target})'/>"),
        );
        assert!(decode(&source).is_err());
        let source = svg(
            "width='4' height='1'",
            &format!("<defs><mask id='m' mask='url({target})'/></defs>"),
        );
        assert!(decode(&source).is_err());
    }
    let source=svg("width='4' height='1'", "<defs><mask id='m'><image href='file:///C:/mask.png'/></mask></defs><rect width='4' height='1' mask='url(#m)'/>");
    assert!(decode(&source).is_err());
}
#[test]
fn surface_svg_masks_reject_three_way_cycles_before_renderer() {
    let mut defs = String::new();
    for (id, next) in [("a", "b"), ("b", "c"), ("c", "a")] {
        defs.push_str(&format!(
            "<mask id='{id}'><rect width='4' height='1' fill='white' mask='url(#{next})'/></mask>"
        ));
    }
    let source = svg(
        "width='4' height='1'",
        &format!("<defs>{defs}</defs><rect width='4' height='1' mask='url(#a)'/>"),
    );
    assert!(decode(&source).is_err());
}
