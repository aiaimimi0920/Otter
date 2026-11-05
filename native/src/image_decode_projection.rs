//! 传统投送的严格 PNG 解码入口；复用 WIC 像素 owner，不改变 Relay 的 JPEG 合同。
use super::*;
impl Image {
    pub(crate) fn from_projection_png(
        bytes: &[u8],
        width: u32,
        height: u32,
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        if bytes.len() > crate::projection_delivery::accept::MAX_PNG
            || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            || width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > crate::projection_delivery::accept::MAX_PIXELS
            || stopped()
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid projection PNG representation",
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
            if decoder.GetContainerFormat()? != GUID_ContainerFormatPng
                || decoder.GetFrameCount()? != 1
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Projection container is not a single PNG",
                ));
            }
            Self::decode_source(
                &factory,
                &decoder.GetFrame(0)?.cast()?,
                Some((width, height)),
                64 * 1024 * 1024,
                stopped,
            )
        }
    }
}
