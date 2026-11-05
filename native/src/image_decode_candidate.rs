//! 候选缩略图只接受内联常见容器；先验尺寸再缩到64，不分配完整源像素。
use super::*;
impl Image {
    pub(crate) fn from_candidate_thumbnail(
        bytes: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 || stopped() {
            return Err(Error::new(
                E_INVALIDARG,
                "Candidate preview exceeds input policy",
            ));
        }
        if !(bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            || bytes.starts_with(&[0xff, 0xd8])
            || bytes.starts_with(b"GIF87a")
            || bytes.starts_with(b"GIF89a")
            || bytes.starts_with(b"BM"))
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Unsupported candidate preview header",
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
            // 不因扩展名/MIME提示打开任意系统codec，也不改变既有WebP隔离路线。
            if ![
                GUID_ContainerFormatPng,
                GUID_ContainerFormatJpeg,
                GUID_ContainerFormatGif,
                GUID_ContainerFormatBmp,
            ]
            .contains(&decoder.GetContainerFormat()?)
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Unsupported candidate preview container",
                ));
            }
            let frame = decoder.GetFrame(0)?;
            let (mut width, mut height) = (0, 0);
            frame.GetSize(&mut width, &mut height)?;
            checked_image_bytes(width, height)
                .filter(|_| width <= 8192 && height <= 8192)
                .ok_or_else(|| {
                    Error::new(E_INVALIDARG, "Candidate preview dimensions exceed policy")
                })?;
            let longest = width.max(height).max(64);
            let scaler = factory.CreateBitmapScaler()?;
            scaler.Initialize(
                &frame,
                (width * 64 / longest).max(1),
                (height * 64 / longest).max(1),
                WICBitmapInterpolationModeFant,
            )?;
            Self::decode_source(&factory, &scaler.cast()?, None, 64 * 64 * 4, stopped)
        }
    }
}
