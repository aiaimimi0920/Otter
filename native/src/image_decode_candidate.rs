//! Candidate thumbnails accept only common inline containers; check dimensions before scaling to 64, without allocating full source pixels.
use super::*;
impl Image {
    pub(crate) fn from_candidate_thumbnail(
        bytes: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        Self::thumbnail(bytes, 4 * 1024 * 1024, stopped)
    }
    /// Frozen captures may lack a separate small preview; relax only encoded size and still scale to 64 before pixel allocation.
    pub(crate) fn from_menu_thumbnail(bytes: &[u8], stopped: &impl Fn() -> bool) -> Result<Self> {
        Self::thumbnail(bytes, 16 * 1024 * 1024, stopped)
    }
    fn thumbnail(bytes: &[u8], encoded_limit: usize, stopped: &impl Fn() -> bool) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > encoded_limit || stopped() {
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
            // Filename-extension/MIME hints cannot activate arbitrary system codecs or change existing WebP isolation.
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
            Self::decode_source(
                &factory,
                &scaler.cast()?,
                super::SizePolicy::Any,
                64 * 64 * 4,
                stopped,
            )
        }
    }
}
