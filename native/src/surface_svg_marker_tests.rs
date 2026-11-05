//! 路径 marker 的 Chrome 155 像素基线与真实受限子进程回归。
use super::*;
#[test]
fn surface_svg_context_paints_do_not_turn_zero_width_into_visible_strokes() {
    // 小画布的 AA 快速裁剪不能因不存在内缩区域而跳过实际描边。
    for (w, h, y, width) in [
        (2, 1, 0.5, 0),
        (2, 1, 0.5, 1),
        (2, 2, 0.5, 1),
        (2, 2, 1.5, 1),
        (3, 3, 1.5, 1),
    ] {
        let image = decode(&svg(
            &format!("width='{w}' height='{h}'"),
            &format!("<path d='M0 {y}H{w}' fill='none' stroke='red' stroke-width='{width}'/>"),
        ))
        .unwrap();
        assert_eq!((image.width, image.height), (w, h));
        for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let expected = if width != 0 && i as u32 / w == y as u32 {
                [0, 0, 255, 255]
            } else {
                [0; 4]
            };
            assert_eq!(pixel, expected, "{w}x{h}, y={y}, width={width}, pixel={i}");
        }
    }
}
#[test]
fn surface_svg_context_paints_preserve_gradient_and_pattern_coordinates() {
    for definition in [
        "<linearGradient id='paint' gradientUnits='userSpaceOnUse' x1='0' y1='0' x2='2' y2='0'><stop offset='.5' stop-color='red'/><stop offset='.5' stop-color='blue'/></linearGradient>",
        "<pattern id='paint' patternUnits='userSpaceOnUse' width='2' height='1'><rect width='1' height='1' fill='red'/><rect x='1' width='1' height='1' fill='blue'/></pattern>",
    ] {
        for content in [
            format!("<defs>{definition}<g id='g'><rect width='2' height='1' fill='context-stroke'/></g></defs><use href='#g' stroke='url(#paint)' stroke-width='0'/>"),
            format!("<defs>{definition}<marker id='m' markerUnits='userSpaceOnUse' markerWidth='2' markerHeight='1'><rect width='2' height='1' fill='context-stroke'/></marker></defs><path d='M1 0H0' stroke='url(#paint)' stroke-width='0' marker-end='url(#m)'/>"),
        ] {
            let image = decode(&svg("width='2' height='1'", &content)).unwrap();
            assert_eq!(&*image.pixels, &[0,0,255,255,255,0,0,255], "{content}");
        }
    }
}
#[test]
fn surface_svg_context_paints_match_browser_marker_and_use_instances() {
    let mut failures = Vec::new();
    let marker = |paint| {
        format!("<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='1' markerHeight='1'><rect width='1' height='1' fill='{paint}'/></marker></defs>")
    };
    for (content, expected) in [
        (format!("{}<path d='M0 0H1' fill='red' marker-end='url(#m)'/>",marker("context-fill")), [0,0,0,0,0,0,255,255]),
        (format!("{}<path d='M0 0H1' stroke='blue' stroke-width='0' marker-end='url(#m)'/>",marker("context-stroke")), [0,0,0,0,255,0,0,255]),
        (format!("{}<path d='M0 0H1' stroke='blue' stroke-opacity='0' marker-end='url(#m)'/>",marker("context-stroke")), [0,0,0,0,255,0,0,255]),
        ("<defs><g id='g'><rect width='1' height='1' fill='context-fill'/></g></defs><use href='#g' fill='red'/><use href='#g' x='1' fill='blue'/>".into(), [0,0,255,255,255,0,0,255]),
        ("<defs><g id='g'><rect width='1' height='1' fill='context-stroke'/></g></defs><use href='#g' stroke='red' stroke-width='0'/><use href='#g' x='1' stroke='blue' stroke-width='0'/>".into(), [0,0,255,255,255,0,0,255]),
        ("<rect width='2' height='1' fill='context-fill'/>".into(), [0;8]),
    ] {
        let image=decode(&svg("width='2' height='1'", &content)).unwrap();
        assert_eq!((image.width,image.height),(2,1));
        if *image.pixels != expected {
            failures.push(format!("{content}: {:?} expected {expected:?}", &*image.pixels));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn surface_svg_markers_match_browser_placement_units_orientation_and_clip() {
    for (attrs, position, path, size, bounds) in [
        (
            "markerUnits='userSpaceOnUse'",
            "marker-end",
            "M2 4H6",
            2,
            [6, 4, 8, 6],
        ),
        (
            "markerUnits='userSpaceOnUse'",
            "marker-start",
            "M2 4H6",
            2,
            [2, 4, 4, 6],
        ),
        (
            "markerUnits='userSpaceOnUse'",
            "marker-mid",
            "M2 4H4H6",
            2,
            [4, 4, 6, 6],
        ),
        (
            "markerUnits='userSpaceOnUse' orient='auto'",
            "marker-end",
            "M6 2V4",
            2,
            [4, 4, 6, 6],
        ),
        (
            "markerUnits='userSpaceOnUse' orient='auto-start-reverse'",
            "marker-start",
            "M4 4H6",
            2,
            [2, 2, 4, 4],
        ),
        (
            "markerUnits='strokeWidth'",
            "marker-end",
            "M2 4H6",
            2,
            [6, 4, 10, 8],
        ),
        (
            "markerUnits='userSpaceOnUse' refX='1' refY='1'",
            "marker-end",
            "M2 4H6",
            2,
            [5, 3, 7, 5],
        ),
        (
            "markerUnits='userSpaceOnUse' viewBox='0 0 4 4'",
            "marker-end",
            "M2 4H6",
            2,
            [6, 4, 7, 5],
        ),
        (
            "markerUnits='userSpaceOnUse' overflow='hidden'",
            "marker-end",
            "M2 4H6",
            4,
            [6, 4, 8, 6],
        ),
        (
            "markerUnits='userSpaceOnUse' overflow='visible'",
            "marker-end",
            "M2 4H6",
            4,
            [6, 4, 10, 6],
        ),
    ] {
        let content = format!("<defs><marker id='m' markerWidth='2' markerHeight='2' {attrs}><rect width='{size}' height='2' fill='red'/></marker></defs><path d='{path}' fill='none' stroke='none' stroke-width='2' {position}='url(#m)'/>");
        let image = decode(&svg("width='12' height='8'", &content)).unwrap();
        assert_eq!((image.width, image.height), (12, 8));
        for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let (x, y) = (i as u32 % 12, i as u32 / 12);
            let inside = x >= bounds[0] && x < bounds[2] && y >= bounds[1] && y < bounds[3];
            assert_eq!(
                pixel,
                if inside {
                    &[0, 0, 255, 255]
                } else {
                    &[0, 0, 0, 0]
                },
                "{content} at {x},{y}"
            );
        }
    }
}
#[test]
fn surface_svg_markers_reject_cycles_and_external_references() {
    for content in [
        "<marker id='m'><path d='M0 0H1' marker-end='url(#m)'/></marker>",
        "<marker id='m'><use href='#p'/></marker><path id='p' d='M0 0H1' marker-start='url(#m)'/>",
        "<marker id='a'><path marker-mid='url(#b)'/></marker><marker id='b'><path marker-end='url(#a)'/></marker>",
        "<g marker-end='url(#m)'><defs><marker id='m'><path d='M0 0H1'/></marker></defs><path d='M0 0H1'/></g>",
        "<path d='M0 0H1' marker-end='url(https://example.invalid/a.svg#m)'/>",
        "<marker id='m'><image href='file:///C:/image.png'/></marker>",
    ] {
        assert!(decode(&svg("width='12' height='8'",content)).is_err(), "{content}");
    }
}
