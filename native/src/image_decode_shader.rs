//! Shader 纹理只接受静态 PNG，保留 straight RGBA；不经 PBGRA 往返或读取路径/网络。
use super::*;

pub(crate) struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: crate::pixels::Pixels,
}
/// COM 归调用线程。PNG alpha 不预乘、不另翻 Y；颜色配置/其它容器尚未获得浏览器等价证明。
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
        let limit = crate::shader_worker_wire::pixel_count(width, height)
            .ok()
            .filter(|count| *count <= remaining)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Shader texture budget exhausted"))?;
        let (width, height, rgba) = Image::decode_pixels(
            &factory,
            &frame.cast()?,
            None,
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
