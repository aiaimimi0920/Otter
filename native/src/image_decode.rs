//! Shared WIC decoding; the calling thread owns COM, and results retain only budgeted pixels, not decoders or file locks.
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

enum SizePolicy {
    Any,
    AtMost(u32),
    Exact(u32, u32),
}

impl Image {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_cancel(path, &|| false)
    }
    pub(crate) fn open_with_cancel(path: &Path, stopped: &impl Fn() -> bool) -> Result<Self> {
        let mut name: Vec<u16> = path.as_os_str().encode_wide().collect();
        if name.contains(&0) {
            return Err(Error::new(E_INVALIDARG, "Image path contains NUL"));
        }
        name.push(0);
        // The host already initialized COM on this thread. Release all decoders/converters on return.
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let decoder = factory.CreateDecoderFromFilename(
                PCWSTR(name.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = decoder.GetFrame(0)?;
            Self::decode_source(
                &factory,
                &frame.cast()?,
                SizePolicy::Any,
                256 * 1024 * 1024,
                stopped,
            )
        }
    }
    pub(crate) fn from_encoded(bytes: &[u8]) -> Result<Self> {
        Self::from_encoded_with_cancel(bytes, &|| false)
    }
    pub(crate) fn from_encoded_with_cancel(
        bytes: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > 257 * 1024 * 1024 {
            return Err(Error::new(
                E_INVALIDARG,
                "Encoded image exceeds input budget",
            ));
        }
        Self::from_encoded_limits(bytes, SizePolicy::Any, 256 * 1024 * 1024, stopped)
    }
    /// Check Relay's declared dimensions before pixel allocation; generic image sniffing must not admit disguised containers.
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
        Self::from_encoded_limits(
            bytes,
            SizePolicy::Exact(width, height),
            256 * 1024 * 1024,
            &|| false,
        )
    }
    pub(crate) fn from_clipboard_history(
        bytes: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > crate::clipboard_history::MAX_IMAGE_BYTES {
            return Err(Error::new(
                E_INVALIDARG,
                "Clipboard image input exceeds budget",
            ));
        }
        Self::from_encoded_limits(bytes, SizePolicy::AtMost(8192), 64 * 1024 * 1024, stopped)
    }
    fn from_encoded_limits(
        bytes: &[u8],
        size: SizePolicy,
        limit: usize,
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            // bytes remain alive until decoders/converters are destroyed; no temporary files or cross-thread COM.
            stream.InitializeFromMemory(bytes)?;
            let decoder = factory.CreateDecoderFromStream(
                &stream,
                std::ptr::null(),
                WICDecodeMetadataCacheOnDemand,
            )?;
            if matches!(size, SizePolicy::Exact(..))
                && decoder.GetContainerFormat()? != GUID_ContainerFormatJpeg
            {
                return Err(Error::new(E_INVALIDARG, "Relay container is not JPEG"));
            }
            let frame = decoder.GetFrame(0)?;
            Self::decode_source(&factory, &frame.cast()?, size, limit, stopped)
        }
    }
    // Surface may supply an orientation-transformed source; ordinary open and Relay retain original-frame/original-dimension contracts.
    unsafe fn decode_source(
        factory: &IWICImagingFactory,
        source: &IWICBitmapSource,
        size: SizePolicy,
        limit: usize,
        stopped: &impl Fn() -> bool,
    ) -> Result<Self> {
        let (width, height, pixels) = unsafe {
            Self::decode_pixels(
                factory,
                source,
                size,
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
    // One owner for pixel allocation/conversion; callers explicitly select the format, never exposing straight RGBA as Image.
    unsafe fn decode_pixels(
        factory: &IWICImagingFactory,
        source: &IWICBitmapSource,
        size: SizePolicy,
        limit: usize,
        format: &windows::core::GUID,
        stopped: &impl Fn() -> bool,
    ) -> Result<(u32, u32, crate::pixels::Pixels)> {
        unsafe {
            let (mut width, mut height) = (0, 0);
            source.GetSize(&mut width, &mut height)?;
            match size {
                SizePolicy::Exact(w, h) if (w, h) != (width, height) => {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Relay JPEG dimensions differ from metadata",
                    ));
                }
                SizePolicy::AtMost(max) if width > max || height > max => {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Image dimensions exceed input policy",
                    ));
                }
                _ => {}
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
pub(crate) fn pixel_buffer(count: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
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
