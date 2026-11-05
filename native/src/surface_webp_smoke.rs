//! 实际产品 EXE 的 child 分流验收；不依赖 libtest 私有入口，不读写用户资源。
pub fn run() -> windows::core::Result<()> {
    use windows::{core::Error, Win32::Foundation::E_FAIL};
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(
            &[255, 0, 0, 255, 0, 255, 0, 128],
            2,
            1,
            image_webp::ColorType::Rgba8,
        )
        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let image = crate::image_decode::Image::from_surface(&bytes, "image/webp", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 255, 255, 0, 128, 0, 128] {
        return Err(Error::new(E_FAIL, "WebP native child smoke mismatch"));
    }
    println!("native-webp-child-smoke: isolated=true pixels=true profile=false network=false");
    // 同一 smoke 验证新增 SVG 私有 CLI，不能仅依赖 libtest 入口。
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="1" height="1" fill="red"/><rect x="1" width="1" height="1" fill="lime" opacity="0.5"/></svg>"#;
    let image = crate::image_decode::Image::from_surface(svg, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 255, 255, 0, 128, 0, 128] {
        return Err(Error::new(E_FAIL, "SVG native child smoke mismatch"));
    }
    println!("native-svg-child-smoke: isolated=true pixels=true profile=false network=false");
    let fractional = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2.5" height="1.5"><rect width="1" height="1" fill="red"/></svg>"#;
    let (image, logical) =
        crate::image_decode::Image::from_surface_sized(fractional, "image/svg+xml", None, || {
            false
        })?;
    if logical != [2.5, 1.5]
        || (image.width, image.height) != (3, 2)
        || image.pixels[..4] != [0, 0, 255, 255]
        || image.pixels[4..].iter().any(|v| *v != 0)
    {
        return Err(Error::new(E_FAIL, "SVG fractional child smoke mismatch"));
    }
    let text = "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='48'><text y='28' font-size='24'>Hello 中文</text></svg>";
    let image =
        crate::image_decode::Image::from_surface(text.as_bytes(), "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (128, 48)
        || image.pixels.chunks_exact(4).filter(|p| p[3] != 0).count() < 100
    {
        return Err(Error::new(E_FAIL, "SVG text child smoke mismatch"));
    }
    // Browser-derived independent glyph positions exercise the actual EXE child,
    // rather than merely accepting a nonempty result from the new attributes.
    let length = concat!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'>",
        "<text font-family='Consolas' font-size='16' fill='red' x='150' y='40' ",
        "direction='rtl' text-anchor='start' textLength='80' lengthAdjust='spacing'>FH</text></svg>"
    );
    let expected = concat!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'>",
        "<text font-family='Consolas' font-size='16' fill='red' x='70' y='40'>F</text>",
        "<text font-family='Consolas' font-size='16' fill='red' x='141.203125' y='40'>H</text></svg>"
    );
    let actual =
        crate::image_decode::Image::from_surface(length.as_bytes(), "image/svg+xml", None, || {
            false
        })?;
    let expected = crate::image_decode::Image::from_surface(
        expected.as_bytes(),
        "image/svg+xml",
        None,
        || false,
    )?;
    if (actual.width, actual.height) != (300, 80) || actual.pixels != expected.pixels {
        return Err(Error::new(
            E_FAIL,
            "SVG textLength/direction child smoke mismatch",
        ));
    }
    println!(
        "native-svg-text-length-smoke: isolated=true pixels=true direction=rtl profile=false network=false"
    );
    let nested = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><svg width="0" height="1"><rect width="2" height="1" fill="blue"/></svg><svg x="1" width="1" height="1"><rect width="5" height="1" fill="red"/></svg></svg>"#;
    let image = crate::image_decode::Image::from_surface(nested, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 0, 0, 0, 0, 255, 255] {
        return Err(Error::new(E_FAIL, "SVG nested child smoke mismatch"));
    }
    let mask = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><defs><mask id="m"><rect width="1" height="1" fill="white"/></mask></defs><rect width="2" height="1" fill="red" mask="url(#m)"/></svg>"#;
    let image = crate::image_decode::Image::from_surface(mask, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 255, 255, 0, 0, 0, 0] {
        return Err(Error::new(E_FAIL, "SVG mask child smoke mismatch"));
    }
    let symbol = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><defs><symbol id="s" viewBox="0 0 1 1"><rect width="1" height="1"/></symbol></defs><use href="#s" width="1" height="1" fill="red"/><use href="#s" x="1" width="1" height="1" fill="blue"/></svg>"##;
    let image = crate::image_decode::Image::from_surface(symbol, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 255, 255, 255, 0, 0, 255] {
        return Err(Error::new(E_FAIL, "SVG symbol child smoke mismatch"));
    }
    let pattern = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><defs><pattern id="p" patternUnits="userSpaceOnUse" width="2" height="1"><rect width="1" height="1" fill="red"/></pattern></defs><rect width="2" height="1" fill="url(#p)"/></svg>"#;
    let image = crate::image_decode::Image::from_surface(pattern, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 255, 255, 0, 0, 0, 0] {
        return Err(Error::new(E_FAIL, "SVG pattern child smoke mismatch"));
    }
    let marker = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="1" markerHeight="1"><rect width="1" height="1" fill="red"/></marker></defs><path d="M0 0H1" fill="none" marker-end="url(#m)"/></svg>"#;
    let image = crate::image_decode::Image::from_surface(marker, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (2, 1) || *image.pixels != [0, 0, 0, 0, 0, 0, 255, 255] {
        return Err(Error::new(E_FAIL, "SVG marker child smoke mismatch"));
    }
    // Stroke first leaves the full interior red, including the inner stroke half.
    let ordered = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect x="2" y="2" width="4" height="4" stroke-width="4" stroke="blue" fill="red" paint-order="stroke fill"/></svg>"#;
    let image = crate::image_decode::Image::from_surface(ordered, "image/svg+xml", None, || false)?;
    if (image.width, image.height) != (8, 8)
        || image.pixels.chunks_exact(4).enumerate().any(|(i, pixel)| {
            let inside = (2..6).contains(&(i % 8)) && (2..6).contains(&(i / 8));
            pixel
                != if inside {
                    &[0, 0, 255, 255]
                } else {
                    &[255, 0, 0, 255]
                }
        })
    {
        return Err(Error::new(E_FAIL, "SVG paint-order child smoke mismatch"));
    }
    println!("native-svg-paint-order-smoke: isolated=true pixels=true profile=false network=false");
    Ok(())
}
