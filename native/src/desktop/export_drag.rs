//! 拖出快照只编码 PNG，不分配剪贴板专用 DIB；复用显示透明度与标注合成。
use super::*;

pub(in crate::desktop) fn png_with_opacity(
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> Result<(crate::pixels::Pixels, crate::desktop::drag_preview::Preview)> {
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = compose_opacity(image, editor, &wic, opacity, crate::pixels::reserve)?;
    let preview = crate::desktop::drag_preview::prepare(&wic, &bitmap.bitmap)?;
    let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap)?;
    drop(bitmap);
    Ok((stream_pixels(stream, size)?, preview))
}
