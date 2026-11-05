//! 共享 WIC 解码；调用线程拥有 COM，结果只保留受预算约束的像素，不保留解码器或文件锁。
use crate::viewport::checked_image_bytes;
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::{Error, Interface, Result, PCWSTR},
    Win32::{
        Foundation::{E_INVALIDARG, GENERIC_READ},
        Graphics::Imaging::*,
        System::Com::*,
    },
};
#[path = "image_decode_bgra.rs"]
mod bgra;
#[path = "image_decode_candidate.rs"]
mod candidate;
#[path = "image_decode_history.rs"]
mod history;
#[path = "image_decode_projection.rs"]
mod projection;
#[path = "image_decode_shader.rs"]
pub(crate) mod shader;
#[path = "image_decode_sticker.rs"]
mod sticker;
#[path = "surface_image_decode.rs"]
mod surface;
#[path = "image_decode_tile.rs"]
mod tile;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: crate::pixels::Pixels,
}

impl Image {
    pub fn open(path: &Path) -> Result<Self> {
        let mut name: Vec<u16> = path.as_os_str().encode_wide().collect();
        if name.contains(&0) {
            return Err(Error::new(E_INVALIDARG, "Image path contains NUL"));
        }
        name.push(0);
        // COM 已由宿主在同一线程初始化。所有 decoder/converter 在返回时释放。
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let decoder = factory.CreateDecoderFromFilename(
                PCWSTR(name.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;
            Self::decode(&factory, &decoder, None, 256 * 1024 * 1024)
        }
    }
    pub(crate) fn from_encoded(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > 257 * 1024 * 1024 {
            return Err(Error::new(
                E_INVALIDARG,
                "Encoded image exceeds input budget",
            ));
        }
        Self::from_encoded_expected(bytes, None)
    }
    /// Relay 的声明尺寸必须在像素分配前核对，不能用通用图片嗅探接受伪装容器。
    pub(crate) fn from_relay_jpeg(bytes: &[u8], width: u32, height: u32) -> Result<Self> {
        if bytes.len() > crate::live_relay_frame::MAX_JPEG_BYTES
            || !bytes.starts_with(&[0xff, 0xd8])
            || !bytes.ends_with(&[0xff, 0xd9])
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid relay JPEG representation",
            ));
        }
        Self::from_encoded_expected(bytes, Some((width, height)))
    }
    fn from_encoded_expected(bytes: &[u8], expected: Option<(u32, u32)>) -> Result<Self> {
        Self::from_encoded_limits(bytes, expected, 256 * 1024 * 1024, u32::MAX)
    }
    pub(crate) fn from_clipboard_history(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > crate::clipboard_history::MAX_IMAGE_BYTES {
            return Err(Error::new(
                E_INVALIDARG,
                "Clipboard image input exceeds budget",
            ));
        }
        Self::from_encoded_limits(bytes, None, 64 * 1024 * 1024, 8192)
    }
    fn from_encoded_limits(
        bytes: &[u8],
        expected: Option<(u32, u32)>,
        limit: usize,
        max_side: u32,
    ) -> Result<Self> {
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            // bytes 在 decoder/converter 销毁前始终存活；没有临时落盘或跨线程 COM。
            stream.InitializeFromMemory(bytes)?;
            let decoder = factory.CreateDecoderFromStream(
                &stream,
                std::ptr::null(),
                WICDecodeMetadataCacheOnDemand,
            )?;
            if expected.is_some() && decoder.GetContainerFormat()? != GUID_ContainerFormatJpeg {
                return Err(Error::new(E_INVALIDARG, "Relay container is not JPEG"));
            }
            if max_side != u32::MAX {
                let frame = decoder.GetFrame(0)?;
                let (mut width, mut height) = (0, 0);
                frame.GetSize(&mut width, &mut height)?;
                if width > max_side || height > max_side {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Image dimensions exceed input policy",
                    ));
                }
            }
            Self::decode(&factory, &decoder, expected, limit)
        }
    }
    unsafe fn decode(
        factory: &IWICImagingFactory,
        decoder: &IWICBitmapDecoder,
        expected: Option<(u32, u32)>,
        limit: usize,
    ) -> Result<Self> {
        unsafe {
            let frame = decoder.GetFrame(0)?;
            Self::decode_source(factory, &frame.cast()?, expected, limit, &|| false)
        }
    }
    // Surface 可以传入方向变换源；普通打开及 Relay 仍使用原始帧和原始尺寸合同。
    unsafe fn decode_source(
        factory: &IWICImagingFactory,
        source: &IWICBitmapSource,
        expected: Option<(u32, u32)>,
        limit: usize,
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        let (width, height, pixels) = unsafe {
            Self::decode_pixels(
                factory,
                source,
                expected,
                limit,
                &GUID_WICPixelFormat32bppPBGRA,
                stopped,
            )?
        };
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
    // 像素分配/转换只有一个 owner；调用方明确选择格式，不把 straight RGBA 暴露为 Image。
    unsafe fn decode_pixels(
        factory: &IWICImagingFactory,
        source: &IWICBitmapSource,
        expected: Option<(u32, u32)>,
        limit: usize,
        format: &windows::core::GUID,
        stopped: &impl Fn() -> bool,
    ) -> Result<(u32, u32, crate::pixels::Pixels)> {
        unsafe {
            let (mut width, mut height) = (0, 0);
            source.GetSize(&mut width, &mut height)?;
            if expected.is_some_and(|size| size != (width, height)) {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Relay JPEG dimensions differ from metadata",
                ));
            }
            let count = checked_image_bytes(width, height)
                .filter(|count| *count <= limit)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Image exceeds native decode budget"))?;
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(
                source,
                format,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )?;
            if stopped() {
                return Err(Error::new(E_INVALIDARG, "Image decode cancelled"));
            }
            let (mut pixels, lease) = pixel_buffer(count)?;
            converter.CopyPixels(std::ptr::null(), width * 4, &mut pixels)?;
            if stopped() {
                return Err(Error::new(E_INVALIDARG, "Image decode cancelled"));
            }
            Ok((
                width,
                height,
                crate::pixels::Pixels::from_reserved(pixels, lease),
            ))
        }
    }
}
fn pixel_buffer(count: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    use windows::Win32::Foundation::E_OUTOFMEMORY;
    let mut lease = crate::pixels::reserve(count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Decoded pixel budget exhausted"))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate image pixels"))?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Pixel allocation capacity exceeds budget",
        ));
    }
    pixels.resize(count, 0);
    Ok((pixels, lease))
}
