//! Wall images reuse WIC/isolated WebP/original RGBA conversion; enforce Tile-specific limits before pixel allocation.
use super::*;
impl Image {
    pub(crate) fn from_tile(
        bytes: &[u8],
        mime: &str,
        dimensions: Option<(u32, u32)>,
        stopped: impl Fn() -> bool,
    ) -> Result<Self> {
        let check = |w: u32, h: u32| -> Result<()> {
            if w == 0
                || h == 0
                || w > 16384
                || h > 16384
                || u64::from(w) * u64::from(h) > crate::tile_wire::MAX_PIXELS
            {
                return Err(Error::new(E_INVALIDARG, "Tile image pixel limit"));
            }
            Ok(())
        };
        if mime == "application/x-neuro-rgba8" {
            let (w, h) =
                dimensions.ok_or_else(|| Error::new(E_INVALIDARG, "Missing Tile dimensions"))?;
            check(w, h)?;
            return Self::from_surface(bytes, mime, dimensions, stopped);
        }
        if mime == "image/webp" {
            let (w, h) = crate::surface_webp_header::inspect(bytes)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid Tile WebP"))?;
            check(w, h)?;
            return Self::from_surface(bytes, mime, Some((w, h)), stopped);
        }
        // Wall's GIF contract uses the first frame; ordinary Surface's single-frame contract and EXIF semantics stay unchanged.
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
            let (mut w, mut h) = (0, 0);
            frame.GetSize(&mut w, &mut h)?;
            check(w, h)?;
            Self::decode_source(
                &factory,
                &frame.cast()?,
                super::SizePolicy::Any,
                64 * 1024 * 1024,
                &stopped,
            )
        }
    }
}
