//! symbol/use 的真实 Job 子进程与 Chrome 155 img→canvas 像素基线。
use super::*;
#[test]
fn surface_svg_symbols_resolve_content_in_each_instance_viewport() {
    let definition =
        "<symbol id='s' viewBox='0 0 10 10'><rect width='50%' height='50%' fill='red'/></symbol>";
    for (content, bounds) in [
        (format!("{definition}<use href='#s' x='10' y='5' width='20' height='10'/>"), vec![[15,5,20,10]]),
        (format!("{}<use href='#s' x='10' y='5' width='20' height='10'/>", definition.replace(" viewBox='0 0 10 10'", "")), vec![[10,5,20,10]]),
        (format!("{definition}<use href='#s' width='20' height='10'/><use href='#s' x='30' width='40' height='20'/>"), vec![[5,0,10,5],[40,0,50,10]]),
        (format!("{definition}<svg x='5' y='5' width='40' height='20'><use href='#s' x='25%' y='25%' width='50%' height='50%'/></svg>"), vec![[20,10,25,15]]),
        (format!("{definition}<defs><use id='u' href='#s' width='20' height='10'/></defs><use href='#u' x='10' y='5'/>"), vec![[15,5,20,10]]),
        ("<symbol id='s' width='0' height='0' viewBox='0 0 10 10'><rect width='10' height='10' fill='red'/></symbol><use href='#s' width='20' height='10'/>".into(), vec![[5,0,15,10]]),
        (definition.to_owned(), vec![]),
        ("<symbol id='s'/><use href='#s'/>".into(), vec![]),
        ("<s:symbol xmlns:s='http://www.w3.org/2000/svg' id='s' viewBox='0 0 10 10'><rect width='10' height='10' fill='red'/></s:symbol><use href='#s' width='20' height='10'/>".into(), vec![[5,0,15,10]]),
        ("<defs><rect id='r' width='10' height='10' fill='red'/></defs><use href='#r' width='0' height='0'/>".into(), vec![[0,0,10,10]]),
        ("<symbol id='s' width='0'/><defs><rect id='r' width='10' height='10' fill='red'/></defs><use xmlns:xlink='http://www.w3.org/1999/xlink' xlink:href='#s' href='#r'/>".into(), vec![[0,0,10,10]]),
        ("<symbol id='s' width='0'><rect width='10' height='10' fill='red'/></symbol><defs><rect id='r' width='10' height='10' fill='red'/></defs><use xmlns:xlink='http://www.w3.org/1999/xlink' xlink:href='#r' href='#s'/>".into(), vec![]),
    ] {
        let image = decode(&svg("width='80' height='40'", &content)).unwrap();
        for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let (x,y) = (i as u32 % 80, i as u32 / 80);
            let inside = bounds.iter().any(|b| x>=b[0] && y>=b[1] && x<b[2] && y<b[3]);
            assert_eq!(pixel, if inside { &[0,0,255,255] } else { &[0,0,0,0] }, "{content} at {x},{y}");
        }
    }
}
#[test]
fn surface_svg_symbols_keep_instance_style_and_transform_independent() {
    let source=svg("width='2' height='1'", "<defs><symbol id='s' viewBox='0 0 1 1'><rect width='1' height='1'/></symbol></defs><use href='#s' width='1' height='1' fill='red'/><use href='#s' x='1' width='1' height='1' fill='blue'/>");
    assert_eq!(
        &*decode(&source).unwrap().pixels,
        &[0, 0, 255, 255, 255, 0, 0, 255]
    );
}
#[test]
fn surface_svg_symbols_match_browser_viewport_and_clipping() {
    let mut failures = Vec::new();
    for (attrs, placement, rect, bounds) in [
        (
            "",
            "x='10' y='5' width='20' height='10'",
            10,
            [15, 5, 25, 15],
        ),
        (
            "preserveAspectRatio='none'",
            "x='10' y='5' width='20' height='10'",
            10,
            [10, 5, 30, 15],
        ),
        (
            "preserveAspectRatio='xMidYMid slice'",
            "x='10' y='5' width='20' height='10'",
            10,
            [10, 5, 30, 15],
        ),
        (
            "",
            "x='10' y='5' width='20' height='10'",
            100,
            [15, 5, 30, 15],
        ),
        (
            "overflow='visible'",
            "x='10' y='5' width='20' height='10'",
            100,
            [15, 5, 80, 40],
        ),
        (
            "",
            "x='25%' y='25%' width='50%' height='50%'",
            10,
            [30, 10, 50, 30],
        ),
        ("", "width='0' height='10'", 10, [0, 0, 0, 0]),
    ] {
        let content = format!("<defs><symbol id='s' viewBox='0 0 10 10' {attrs}><rect width='{rect}' height='{rect}' fill='red'/></symbol></defs><use href='#s' {placement}/>");
        let source = svg("width='80' height='40'", &content);
        let (image, logical) =
            Image::from_surface_sized(source.as_bytes(), "image/svg+xml", None, || false).unwrap();
        assert_eq!([image.width, image.height], [80, 40]);
        assert_eq!(logical, [80., 40.]);
        for (i, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let (x, y) = (i as u32 % 80, i as u32 / 80);
            let inside = x >= bounds[0] && x < bounds[2] && y >= bounds[1] && y < bounds[3];
            let expected = if inside {
                &[0, 0, 255, 255]
            } else {
                &[0, 0, 0, 0]
            };
            if pixel != expected {
                failures.push(format!(
                    "{content} at {x},{y}: {pixel:?} expected {expected:?}"
                ));
                break; // 记录该案例首个差异，继续检查其余案例；任何差异仍令整个测试失败。
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn surface_svg_symbols_reject_cycles_and_external_references() {
    for content in [
        "<defs><symbol id='a'><use href='#b'/></symbol><symbol id='b'><use href='#c'/></symbol><symbol id='c'><use href='#a'/></symbol></defs><use href='#a'/>",
        "<defs><g id='a'><use href='#b'/></g><g id='b'><use href='#a'/></g></defs><use href='#a'/>",
        "<symbol id='s'><use href='https://example.invalid/icon.svg#s'/></symbol><use href='#s'/>",
        "<symbol id='s'><image href='file:///C:/image.png'/></symbol><use href='#s'/>",
    ] {
        assert!(decode(&svg("width='80' height='40'",content)).is_err());
    }
}
