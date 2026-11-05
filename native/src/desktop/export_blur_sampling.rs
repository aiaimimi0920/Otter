//! 主干模糊标注的独立取样合同：忽略底图翻转/传播框，逻辑裁剪直接使用源像素偏移。
use super::*;

pub(super) fn original(image: &Image, flipped: (bool, bool), cancel: &AtomicBool) -> Result<Image> {
    // 只在首次实际模糊资源请求时调用；无标注时不能为撤销底图翻转额外分配整图。
    let pixels = crate::raster_flip::image_axes(
        &image.pixels,
        (image.width, image.height),
        flipped.0,
        flipped.1,
        || cancel.load(Ordering::Acquire),
    )
    .map_err(|error| {
        Error::new(
            match error {
                crate::raster_flip::Error::Cancelled => windows::Win32::Foundation::E_ABORT,
                crate::raster_flip::Error::BudgetExceeded => {
                    windows::Win32::Foundation::E_OUTOFMEMORY
                }
                _ => E_INVALIDARG,
            },
            "Cannot restore blur source orientation",
        )
    })?;
    Ok(Image {
        width: image.width,
        height: image.height,
        pixels,
    })
}
