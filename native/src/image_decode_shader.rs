//! Shader textures accept only static PNG and retain straight RGBA; no PBGRA round trips or path/network reads.
use super::*;

pub(crate) struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: crate::pixels::Pixels,
}
/// COM belongs to the calling thread. PNG alpha is not premultiplied or Y-flipped; color profiles/other containers lack browser-equivalence proof.
pub(crate) fn png(bytes: &[u8], remaining: usize, stopped: &impl Fn() -> bool) -> Result<Texture> {
    if bytes.is_empty()
        || bytes.len() > 64 * 1024 * 1024
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || stopped()
        || surface::animated_png(bytes)?
    {
        return Err(Error::new(E_INVALIDARG, "Invalid or cancelled Shader PNG"));
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
        if decoder.GetContainerFormat()? != GUID_ContainerFormatPng || decoder.GetFrameCount()? != 1
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Shader texture is not a static PNG",
            ));
        }
        let frame = decoder.GetFrame(0)?;
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height)?;
        let limit = crate::viewport::checked_shader_texture_bytes(width, height)
            .filter(|count| *count <= remaining)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Shader texture budget exhausted"))?;
        let (width, height, rgba) = Image::decode_pixels(
            &factory,
            &frame.cast()?,
            super::SizePolicy::Any,
            limit,
            &GUID_WICPixelFormat32bppRGBA,
            stopped,
        )?;
        Ok(Texture {
            width,
            height,
            rgba,
        })
    }
}
