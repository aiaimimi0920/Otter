//! 工作流底图的有预算 D2D 取样与可取消读回；不合成标注、不提前应用透明度。
use super::*;
#[cfg(test)]
#[path = "export_source_sample_tests.rs"]
mod tests;

pub(super) fn readback(image: &Image, layout: &Layout, cancel: &AtomicBool) -> Result<Image> {
    check(cancel)?;
    if layout.border.is_some() || layout.radius != 0.0 {
        return Err(Error::new(
            E_INVALIDARG,
            "Source sampling must precede frame decoration",
        ));
    }
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = compose_layout(
        image,
        &Editor::default(),
        &wic,
        1.0,
        crate::pixels::reserve,
        Some(layout),
    )?;
    check(cancel)?;
    let count = checked_image_bytes(layout.output.0, layout.output.1)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid sampled base size"))?;
    let allocation = || {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Sampled base budget exhausted",
        )
    };
    let mut lease = crate::pixels::reserve(count).ok_or_else(allocation)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| allocation())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(allocation());
    }
    bytes.resize(count, 0);
    for (y, row) in bytes
        .chunks_exact_mut(layout.output.0 as usize * 4)
        .enumerate()
    {
        check(cancel)?;
        let rect = WICRect {
            X: 0,
            Y: y as i32,
            Width: layout.output.0 as i32,
            Height: 1,
        };
        unsafe {
            bitmap.bitmap.CopyPixels(&rect, layout.output.0 * 4, row)?;
        }
    }
    drop(bitmap);
    check(cancel)?;
    Ok(Image {
        width: layout.output.0,
        height: layout.output.1,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    })
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Sampled base cancelled",
        ))
    } else {
        Ok(())
    }
}
