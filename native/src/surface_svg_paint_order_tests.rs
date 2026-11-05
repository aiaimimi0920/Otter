//! 浏览器验证的显式分层等价关系；经过生产准入与真实 Job child。
#[test]
fn paint_order_partial_tokens_and_normal_match_explicit_completion() {
    for (short, full) in [
        ("normal", "fill stroke markers"),
        ("fill", "fill stroke markers"),
        ("stroke", "stroke fill markers"),
        ("markers", "markers fill stroke"),
        ("fill stroke", "fill stroke markers"),
        ("fill markers", "fill markers stroke"),
        ("stroke fill", "stroke fill markers"),
        ("stroke markers", "stroke markers fill"),
        ("markers fill", "markers fill stroke"),
        ("markers stroke", "markers stroke fill"),
        ("  stroke &#x9; markers &#xA; fill  ", "stroke markers fill"),
    ] {
        let wrap = |order| {
            super::svg(
                "width='80' height='64'",
                &format!(
                    "<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)' paint-order='{order}'/>"
                ),
            )
        };
        let actual = super::decode(&wrap(short)).unwrap();
        let expected = super::decode(&wrap(full)).unwrap();
        assert_eq!(actual.pixels, expected.pixels, "{short}");
    }
}

#[test]
fn paint_order_invalid_tokens_fail_closed_and_worker_recovers() {
    for order in [
        "",
        " ",
        "none",
        "inherit",
        "Fill",
        "stroke,fill",
        "fill fill",
        "markers stroke markers",
        "normal fill",
        "fill normal",
        "normal normal",
        "fill stroke markers extra",
        "fillstroke",
        "fill&#xA0;stroke",
    ] {
        let source = super::svg(
            "width='80' height='64'",
            &format!("<path d='M12 12H48V40' paint-order='{order}'/>"),
        );
        assert!(super::decode(&source).is_err(), "accepted {order}");
    }
    assert!(super::decode(&super::svg(
        "width='2' height='2'",
        "<rect width='2' height='2' style='paint-order:stroke'/>"
    ))
    .is_err());
    let image = super::decode(&super::svg(
        "width='2' height='2'",
        "<rect width='2' height='2' fill='red' paint-order='normal'/>",
    ))
    .unwrap();
    assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
}

#[test]
fn paint_order_path_fill_stroke_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='fill stroke markers'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_fill_stroke_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='fill stroke markers' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_fill_stroke_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='fill stroke markers'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_fill_stroke_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='fill stroke markers' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_path_fill_markers_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='fill markers stroke'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_fill_markers_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='fill markers stroke' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_fill_markers_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='fill markers stroke'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_fill_markers_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='fill markers stroke' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_path_stroke_fill_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='stroke fill markers'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_stroke_fill_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='stroke fill markers' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_stroke_fill_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='stroke fill markers'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_stroke_fill_markers() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='stroke fill markers' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_path_stroke_markers_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='stroke markers fill'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_stroke_markers_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='stroke markers fill' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_stroke_markers_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='stroke markers fill'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_stroke_markers_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='stroke markers fill' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_path_markers_fill_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='markers fill stroke'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_markers_fill_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='markers fill stroke' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_markers_fill_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='markers fill stroke'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_markers_fill_stroke() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='markers fill stroke' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_path_markers_stroke_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='markers stroke fill'  fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_opacity_markers_stroke_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><path d='M12 12H48V40' stroke-width='8' paint-order='markers stroke fill' opacity='0.5' fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g opacity='0.5'><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_inherited_markers_stroke_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='12' markerHeight='12' refX='6' refY='6'><rect width='12' height='12' fill='blue'/></marker></defs><g paint-order='markers stroke fill'><path d='M12 12H48V40' stroke-width='8'   fill='red' stroke='lime' marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></g>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<g ><rect x='6' y='6' width='12' height='12' fill='blue'/><rect x='42' y='6' width='12' height='12' fill='blue'/><rect x='42' y='34' width='12' height='12' fill='blue'/><path d='M12 12H48V40' stroke-width='8' fill='none' stroke='lime'/><path d='M12 12H48V40' stroke-width='8' fill='red' stroke='none'/></g>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
#[test]
fn paint_order_text_markers_stroke_fill() {
    let actual = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' paint-order='markers stroke fill' fill='red' stroke='lime'>H</text>"###)).unwrap();
    let expected = super::decode(&super::svg("width='80' height='64'", r###"<text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='none' stroke='lime'>H</text><text x='20' y='48' font-family='Consolas' font-size='40' stroke-width='8' fill='red' stroke='none'>H</text>"###)).unwrap();
    assert_eq!((actual.width, actual.height), (80, 64));
    assert!(expected.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert_eq!(actual.pixels, expected.pixels);
}
