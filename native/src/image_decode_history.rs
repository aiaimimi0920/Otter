//! 旧history.json可能含原图fallback；先限制容器与声明尺寸，再由WIC缩到240。
use super::*;
impl Image {
    pub(crate) fn from_capture_history(bytes: &[u8], declared_png: bool) -> Result<(Self, bool)> {
        if bytes.is_empty() || bytes.len() > crate::capture_history::MAX_BYTES {
            return Err(Error::new(
                E_INVALIDARG,
                "History image exceeds input budget",
            ));
        }
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
            crate::viewport::checked_image_bytes(width, height).ok_or_else(|| {
                Error::new(E_INVALIDARG, "History source dimensions exceed policy")
            })?;
            let (w, h) = crate::capture_history::image::size(width, height)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid history dimensions"))?;
            let unscaled = (w, h) == (width, height);
            let source = if unscaled {
                frame.cast::<IWICBitmapSource>()?
            } else {
                let scaler = factory.CreateBitmapScaler()?;
                scaler.Initialize(&frame, w, h, WICBitmapInterpolationModeFant)?;
                scaler.cast()?
            };
            let image = Self::decode_source(
                &factory,
                &source,
                super::SizePolicy::Any,
                (crate::capture_history::image::EDGE.pow(2) * 4) as usize,
                &|| false,
            )?;
            let reusable_png = unscaled
                && declared_png
                && decoder
                    .GetContainerFormat()
                    .is_ok_and(|format| format == GUID_ContainerFormatPng)
                && decoder.GetFrameCount().is_ok_and(|count| count == 1);
            Ok((image, reusable_png))
        }
    }
}
