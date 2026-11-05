//! 贴图上传专属尺寸合同；100MP上限不改变其他解码入口或512MiB共享像素总预算。
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
impl Image {
    pub(crate) fn from_sticker(bytes: &[u8], cancel: &AtomicBool) -> Result<Self> {
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            stream.InitializeFromMemory(bytes)?;
            let decoder = factory.CreateDecoderFromStream(
                &stream,
                std::ptr::null(),
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = decoder.GetFrame(0)?;
            let (mut width, mut height) = (0, 0);
            frame.GetSize(&mut width, &mut height)?;
            if width == 0
                || height == 0
                || width > 32768
                || height > 32768
                || u64::from(width) * u64::from(height) > 100_000_000
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Sticker image dimensions exceed policy",
                ));
            }
            Self::decode_source(&factory, &frame.cast()?, None, 400_000_000, &|| {
                cancel.load(Ordering::Acquire)
            })
        }
    }
}
