//! 实际 Job SVG child 的尺寸、像素及拒绝合同；不在父进程解析 XML。
use super::*;
#[path = "surface_svg_bidi_tests.rs"]
mod bidi;
#[path = "surface_svg_depth_tests.rs"]
mod depth;
#[path = "surface_svg_direction_tests.rs"]
mod direction;
#[path = "surface_svg_length_cluster_tests.rs"]
mod length_clusters;
#[path = "surface_svg_length_combined_tests.rs"]
mod length_combined;
#[path = "surface_svg_length_policy_tests.rs"]
mod length_policy;
#[path = "surface_svg_length_units_tests.rs"]
mod length_units;
#[path = "surface_svg_marker_tests.rs"]
mod markers;
#[path = "surface_svg_mask_tests.rs"]
mod masks;
#[path = "surface_svg_paint_order_tests.rs"]
mod paint_order;
#[path = "surface_svg_pattern_tests.rs"]
mod patterns;
#[path = "surface_svg_symbol_tests.rs"]
mod symbols;
#[path = "surface_svg_text_length_tests.rs"]
mod text_length;
#[path = "surface_svg_text_overlap_tests.rs"]
mod text_overlap;
#[path = "surface_svg_text_path_tests.rs"]
mod text_paths;
fn svg(attributes: &str, content: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {attributes}>{content}</svg>"#)
}
fn decode(source: &str) -> Result<Image> {
    Image::from_surface(source.as_bytes(), "image/svg+xml", Some((999, 999)), || {
        false
    })
}
#[test]
fn surface_svg_child_matches_measured_natural_sizes_and_coverage() {
    for (attrs, size, coverage) in [
        (r#"width="80" height="40""#, (80, 40), 3200),
        (r#"viewBox="0 0 80 40""#, (300, 150), 45000),
        ("", (300, 150), 3200),
        (r#"width="80""#, (80, 150), 3200),
        (r#"height="40""#, (300, 40), 3200),
        (r#"width="50%" height="50%""#, (300, 150), 12800),
        (r#"width="1in" height="0.5in""#, (96, 48), 3200),
        (r#"viewBox="0 0 40 40""#, (150, 150), 22500),
        (r#"viewBox="0 0 40 80""#, (75, 150), 5625),
        (r#"viewBox="0 0 120 40""#, (300, 100), 20000),
    ] {
        let image = decode(&svg(attrs, r#"<rect width="80" height="40" fill="red"/>"#)).unwrap();
        assert_eq!((image.width, image.height), size, "{attrs}");
        assert_eq!(
            image.pixels.chunks_exact(4).filter(|p| p[3] != 0).count(),
            coverage,
            "{attrs}"
        );
    }
}
#[test]
fn surface_svg_child_swaps_channels_without_double_premultiplication() {
    let image = decode(&svg(r#"width="2" height="1""#, r#"<rect width="1" height="1" fill="red"/><rect x="1" width="1" height="1" fill="lime" opacity="0.5"/>"#)).unwrap();
    assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    assert!(Image::from_surface(b"ignored", "image/svg+xml", None, || true).is_err());
}
#[test]
fn surface_svg_nested_viewports_match_browser_pixel_rectangles() {
    // Chrome 155 的实际 img -> canvas 基线；逐像素检查位置和透明区，而非只数非零像素。
    for (content, bounds) in [
        ("<svg x='10' y='5' width='20' height='10'><rect width='100' height='100' fill='red'/></svg>", [10,5,30,15]),
        ("<svg x='10' y='5' width='20' height='10' overflow='visible'><rect width='100' height='100' fill='red'/></svg>", [10,5,80,40]),
        ("<svg x='10' y='5' width='20' height='10' viewBox='0 0 10 10'><rect width='10' height='10' fill='red'/></svg>", [15,5,25,15]),
        ("<svg x='10' y='5' width='20' height='10' viewBox='0 0 10 10' preserveAspectRatio='none'><rect width='10' height='10' fill='red'/></svg>", [10,5,30,15]),
        ("<svg x='10' y='5' width='20' height='10' viewBox='0 0 10 10' preserveAspectRatio='xMidYMid slice'><rect width='10' height='10' fill='red'/></svg>", [10,5,30,15]),
        ("<svg x='25%' y='25%' width='50%' height='50%'><rect width='100%' height='100%' fill='red'/></svg>", [20,10,60,30]),
        ("<svg x='10' y='5' width='20' height='10'><svg x='5' y='2' width='10' height='4'><rect width='100%' height='100%' fill='red'/></svg></svg>", [15,7,25,11]),
        ("<svg width='0' height='10'><rect width='100' height='100' fill='red'/></svg>", [0,0,0,0]),
        // 同一零尺寸语义的属性替换/百分比与多层回归，不另称独立浏览器样本。
        ("<svg width='20' height='0%' display='inline'><rect width='100' height='100' fill='red'/></svg>", [0,0,0,0]),
        ("<svg width='0' height='10'><svg width='0' height='10'><rect width='100' height='100' fill='red'/></svg></svg>", [0,0,0,0]),
        ("<svg x='10' y='5' width='20' height='10' overflow='hidden'><rect width='100' height='100' fill='red'/></svg>", [10,5,30,15]),
        ("<svg x='10' y='5' width='20' height='10' overflow='scroll'><rect width='100' height='100' fill='red'/></svg>", [10,5,30,15]),
        ("<svg x='10' y='5' width='20' height='10' overflow='auto'><rect width='100' height='100' fill='red'/></svg>", [10,5,80,40]),
    ] {
        let source = svg("width='80' height='40'", content);
        let (image, logical) = Image::from_surface_sized(source.as_bytes(), "image/svg+xml", None, || false).unwrap();
        assert_eq!([image.width, image.height], [80,40]);
        assert_eq!(logical, [80.,40.]);
        for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let (x,y) = (i as u32 % 80, i as u32 / 80);
            let inside = x >= bounds[0] && x < bounds[2] && y >= bounds[1] && y < bounds[3];
            assert_eq!(pixel, if inside { &[0,0,255,255] } else { &[0,0,0,0] }, "{content} at {x},{y}");
        }
    }
}
#[test]
fn surface_svg_text_child_renders_named_generic_fallback_and_color_glyphs() {
    for (family, text) in [
        ("Microsoft YaHei", "Hello 中文"),
        ("Arial", "中文"),
        ("serif", "Hello"),
        ("monospace", "Hello"),
        ("cursive", "Hello"),
        ("fantasy", "Hello"),
        ("Arial", "العربية"),
        ("Arial", "नमस्ते"),
        ("Arial", "😀"),
        ("Arial", "👩‍💻"),
    ] {
        let content =
            format!("<text x='0' y='32' font-size='24' font-family='{family}'>{text}</text>");
        let image = decode(&svg("width='256' height='64'", &content))
            .unwrap_or_else(|e| panic!("{family}/{text}: {e}"));
        let painted = image.pixels.chunks_exact(4).filter(|p| p[3] != 0).count();
        assert!(painted > 20, "missing text {family}/{text}: {painted}");
        if text.contains('😀') || text.contains('👩') {
            assert!(
                image
                    .pixels
                    .chunks_exact(4)
                    .any(|p| p[0] != p[1] || p[1] != p[2]),
                "missing color glyph"
            );
        }
    }
    let text = "<g fill='red'><text x='0' y='30' font-size='24'>Hello<tspan dx='5' fill='blue'>中</tspan></text></g>";
    let image = decode(&svg("width='256' height='64'", text)).unwrap();
    assert!(image.pixels.chunks_exact(4).any(|p| p[2] > 0 && p[0] == 0));
    assert!(image.pixels.chunks_exact(4).any(|p| p[0] > 0 && p[2] == 0));
}
#[test]
fn surface_svg_text_child_rejects_missing_glyph_and_unrepresented_simulation() {
    for text in [
        "<text y='30'>\u{10ffff}</text>",
        "<text y='30' font-family='SimSun' font-weight='bold'>Hello</text>",
    ] {
        assert!(decode(&svg("width='128' height='48'", text)).is_err());
    }
}
#[test]
fn surface_svg_child_rejects_unimplemented_or_external_content() {
    for content in [
        "<text><textPath path='M0 0H10'>Hello</textPath></text>",
        "<foreignObject/>",
        "<animate/>",
        "<animateMotion/>",
        "<script/>",
        "<style>@import 'https://example.invalid/style';</style>",
        "<image href='file:///C:/sentinel.png'/>",
        "<feImage href='data:image/png,abc'/>",
        // 接入 renderer 补丁不等于滤镜准入；含未引用定义也必须整体拒绝。
        "<filter/>",
        "<defs><filter id='f'><feFlood flood-color='red'/></filter></defs>",
        "<filter><feGaussianBlur stdDeviation='1'/></filter>",
        "<filter><feMorphology radius='1e39'/></filter>",
        "<filter><feTurbulence baseFrequency='1e39' numOctaves='4294967295'/></filter>",
        "<rect width='2' height='1' filter='url(#f)'/>",
        "<use href='https://example.invalid/svg#x'/>",
        "<rect width='2' height='1' style='fill:red'/>",
        "<rect width='2' height='1' fill='url(file:///C:/sentinel.svg#x)'/>",
        "<rect width='2' height='1' fill='URL(https://example.invalid/x)'/>",
        "<rect onclick='doSomething()'/>",
    ] {
        assert!(
            decode(&svg(r#"width="2" height="1""#, content)).is_err(),
            "{content}"
        );
        assert!(
            decode(&svg(
                "width='2' height='1'",
                &format!("<svg>{content}</svg>")
            ))
            .is_err(),
            "nested {content}"
        );
    }
    for source in [
        "<!DOCTYPE svg [<!ENTITY x 'red'>]><svg xmlns='http://www.w3.org/2000/svg' fill='&x;'/>",
        "<?xml-stylesheet href='file:///C:/style.css'?><svg xmlns='http://www.w3.org/2000/svg'/>",
        "<svg xmlns='not-svg'/>",
    ] {
        assert!(decode(source).is_err(), "{source}");
    }
    for attrs in [
        "width='0'",
        "width='16385'",
        "width='4096' height='4097'",
        "width='2em'",
        "viewBox='0 0 0 2'",
    ] {
        assert!(decode(&svg(attrs, "")).is_err(), "{attrs}");
    }
}
#[test]
fn surface_svg_child_bounds_xml_nodes_and_rejects_external_xlink() {
    for width in ["1e-100%", "1e-40%", "0%"] {
        assert!(decode(&svg(
            &format!("width='{width}' height='100%'"),
            "<rect width='1' height='1'/>"
        ))
        .is_err());
    }
    assert!(decode(&svg("width='2' height='1'", &"<g/>".repeat(100_001))).is_err());
    for href in [
        "file:///C:/sentinel.svg",
        "\\\\server\\share\\x.svg",
        "https://example.invalid/x.svg",
        "data:image/svg+xml,abc",
    ] {
        let source = svg(
            "xmlns:xlink='http://www.w3.org/1999/xlink' width='2' height='1'",
            &format!("<use href='#ok' xlink:href='{href}'/>"),
        );
        assert!(decode(&source).is_err());
    }
    assert!(Image::from_surface(&[0x1f, 0x8b, 8], "image/svg+xml", None, || false).is_err());
}
#[test]
fn surface_svg_child_preserves_local_gradient_and_clip_references() {
    let source = svg("width='2' height='1'", "<defs><linearGradient id='p'><stop offset='0' stop-color='red'/><stop offset='1' stop-color='red'/></linearGradient><clipPath id='c'><rect width='1' height='1'/></clipPath></defs><rect width='2' height='1' fill='url(#p)' clip-path='url(#c)'/>");
    assert_eq!(
        &*decode(&source).unwrap().pixels,
        &[0, 0, 255, 255, 0, 0, 0, 0]
    );
}
#[test]
fn surface_svg_fractional_child_preserves_geometry_instead_of_stretching_pixels() {
    for (attrs, raster, logical) in [
        ("width='2.5' height='1.5'", [3, 2], [2.5, 1.5]),
        ("width='2.9' height='1.9'", [3, 2], [2.9, 1.9]),
        ("width='1.4' height='1'", [2, 1], [1.4, 1.]),
        (
            "width='1cm' height='1mm'",
            [38, 4],
            [96. / 2.54, 96. / 25.4],
        ),
    ] {
        let source = svg(attrs, "<rect width='1' height='1' fill='red'/>");
        let (image, size) =
            Image::from_surface_sized(source.as_bytes(), "image/svg+xml", None, || false).unwrap();
        assert_eq!([image.width, image.height], raster);
        for axis in 0..2 {
            assert!((size[axis] - logical[axis]).abs() < 0.00001);
        }
        assert_eq!(&image.pixels[..4], &[0, 0, 255, 255]);
        assert!(
            image.pixels[4..].iter().all(|v| *v == 0),
            "integer 1px geometry must not grow: {attrs}"
        );
    }
}
#[test]
fn surface_svg_fractional_ratio_is_not_rounded_to_the_raster_aspect() {
    for (attrs, raster, size) in [
        ("width='80.5' viewBox='0 0 2 1'", [81, 41], [80.5, 40.25]),
        ("viewBox='0 0 7 3'", [300, 129], [300., 900. / 7.]),
        (
            "width='50%' height='50%' viewBox='0 0 7 3'",
            [300, 129],
            [300., 900. / 7.],
        ),
    ] {
        let source = svg(attrs, "<rect width='1' height='1' fill='red'/>");
        let (image, actual) =
            Image::from_surface_sized(source.as_bytes(), "image/svg+xml", None, || false).unwrap();
        assert_eq!([image.width, image.height], raster);
        assert_eq!(actual, size);
    }
}
