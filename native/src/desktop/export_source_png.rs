//! 源分辨率编码的最终出口；发送合同先限源/内框/外框，再预占 HGlobal 编码预算。
use super::*;

pub(super) struct Output {
    pub size: (u32, u32),
    projection: bool,
}
impl Output {
    pub fn new(image: &Image, frame: &Frame, layout: &Layout) -> Result<Self> {
        let padding = frame.beautify.as_ref().map_or(0, |value| value.padding);
        let twice = padding
            .checked_mul(2)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Source PNG padding overflow"))?;
        let size = (
            layout.output.0.checked_add(twice),
            layout.output.1.checked_add(twice),
        );
        let (Some(width), Some(height)) = size else {
            return Err(Error::new(E_INVALIDARG, "Source PNG dimensions overflow"));
        };
        checked_image_bytes(width, height)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid source PNG dimensions"))?;
        if frame.projection {
            for (width, height) in [(image.width, image.height), layout.output, (width, height)] {
                if width > 8192
                    || height > 8192
                    || u64::from(width) * u64::from(height)
                        > crate::projection_delivery::accept::MAX_PIXELS
                {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Projection dimensions exceed limit",
                    ));
                }
            }
        }
        Ok(Self {
            size: (width, height),
            projection: frame.projection,
        })
    }
    pub fn bitmap(
        &self,
        wic: &IWICImagingFactory,
        bitmap: ComposedImage,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        self.bitmap_with_budget(wic, bitmap, cancel, crate::pixels::reserve, stream_pixels)
    }
    fn bitmap_with_budget(
        &self,
        wic: &IWICImagingFactory,
        bitmap: ComposedImage,
        cancel: &AtomicBool,
        reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
        copy: impl FnOnce(IStream, u64) -> Result<crate::pixels::Pixels>,
    ) -> Result<crate::pixels::Pixels> {
        check(cancel)?;
        let (mut width, mut height) = (0, 0);
        unsafe { bitmap.bitmap.GetSize(&mut width, &mut height)? };
        if self.size != (width, height) {
            return Err(Error::new(E_INVALIDARG, "Source PNG output size changed"));
        }
        // 最坏 scanline/deflate/chunk 占用先于 WIC 编码；Pixels 副本另有原共享预算。
        let raw = (u64::from(width) * 4 + 1) * u64::from(height);
        let bound = raw + raw.div_ceil(16_384) * 64 + 64 * 1024;
        let _encoded = if self.projection {
            Some(reserve(bound as usize).ok_or_else(|| {
                Error::new(
                    windows::Win32::Foundation::E_OUTOFMEMORY,
                    "Projection encoding budget exhausted",
                )
            })?)
        } else {
            None
        };
        let (stream, bytes) = encode_bitmap(wic, &bitmap.bitmap)?;
        // 返回 PNG 再申请预算前释放合成输出，保留普通 workflow 原有准入条件。
        // 发送的 HGlobal 编码租约必须继续存活，直到 stream 拷贝完成或失败。
        drop(bitmap);
        if self.projection && bytes > crate::projection_delivery::accept::MAX_PNG as u64 {
            return Err(Error::new(E_INVALIDARG, "Projection PNG exceeds limit"));
        }
        check(cancel)?;
        let pixels = copy(stream, bytes)?;
        check(cancel)?;
        Ok(pixels)
    }
    pub fn raw(
        &self,
        wic: &IWICImagingFactory,
        image: &Image,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        if !self.projection {
            return super::super::png_raw(image, cancel);
        }
        check(cancel)?;
        let _copy = crate::pixels::reserve(image.pixels.len()).ok_or_else(|| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Projection wrapped bitmap budget exhausted",
            )
        })?;
        let bitmap = unsafe {
            wic.CreateBitmapFromMemory(
                image.width,
                image.height,
                &GUID_WICPixelFormat32bppPBGRA,
                image.width * 4,
                &image.pixels,
            )?
        };
        self.bitmap(
            wic,
            ComposedImage {
                bitmap,
                _lease: _copy,
            },
            cancel,
        )
    }
}
#[cfg(test)]
#[path = "export_source_png_budget_tests.rs"]
mod tests;
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Source PNG encoding cancelled",
        ))
    } else {
        Ok(())
    }
}
