//! Manual export formats consume the same immutable, premultiplied display composite.
use super::*;
use crate::pixels::Pixels;
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::Foundation::{E_ABORT, E_OUTOFMEMORY};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::desktop) enum Format {
    Png,
    Jpeg(u8),
    WebpLossless,
    WebpLossy(u8),
}
impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg(_) => "jpg",
            Self::WebpLossless | Self::WebpLossy(_) => "webp",
        }
    }
    pub fn accepts(self, path: &Path) -> bool {
        path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
            v.eq_ignore_ascii_case(self.ext())
                || matches!(self, Self::Jpeg(_)) && v.eq_ignore_ascii_case("jpeg")
        })
    }
}
pub(in crate::desktop) struct Source {
    pub image: Image,
    pub editor: Editor,
    pub style: style::Snapshot,
    pub opacity: f32,
}
fn budget() -> Error {
    Error::new(E_OUTOFMEMORY, "Image encoding budget exhausted")
}
fn check(stop: &AtomicBool) -> Result<()> {
    if stop.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Export cancelled"))
    } else {
        Ok(())
    }
}
impl Source {
    pub fn encode(&self, fmt: Format, stop: &AtomicBool) -> Result<Pixels> {
        check(stop)?;
        let _apt = Apartment::new()?;
        let wic: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        let composed = self.style.compose(
            &self.image,
            &self.editor,
            &wic,
            self.opacity,
            crate::pixels::reserve,
        )?;
        let (mut w, mut h) = (0, 0);
        unsafe {
            composed.bitmap.GetSize(&mut w, &mut h)?;
        }
        let n = checked_image_bytes(w, h).ok_or_else(budget)?;
        check(stop)?;
        let workspace = match fmt {
            Format::Png => n + h as usize + n.div_ceil(16384) * 64 + 65536,
            Format::Jpeg(1..=100) => n
                .checked_mul(2)
                .and_then(|v| v.checked_add(65536))
                .ok_or_else(budget)?,
            Format::Jpeg(_) => return Err(Error::new(E_INVALIDARG, "Invalid JPEG quality")),
            // VP8 owns padded source/reconstruction planes and assembles both
            // partitions, the frame and RIFF in memory. Charge padded dimensions
            // even for one-pixel-wide images; transfer the final allocation.
            Format::WebpLossy(1..=100)
                if w <= tiny_webp::MAX_DIMENSION && h <= tiny_webp::MAX_DIMENSION =>
            {
                (w.div_ceil(16) as usize)
                    .checked_mul(h.div_ceil(16) as usize)
                    .and_then(|v| v.checked_mul(16 * 16 * 32))
                    .and_then(|v| v.checked_add(1024 * 1024))
                    .ok_or_else(budget)?
            }
            Format::WebpLossy(_) => {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Invalid lossy WebP dimensions or quality",
                ))
            }
            // VP8L builds a full transformed copy and a growing internal bitstream.
            Format::WebpLossless => n
                .checked_mul(5)
                .and_then(|v| v.checked_add(512 * 1024))
                .ok_or_else(budget)?,
        };
        let mut encoding = crate::pixels::reserve(workspace).ok_or_else(budget)?;
        if fmt == Format::Png {
            let (stream, size) = encode_bitmap(&wic, &composed.bitmap)?;
            drop(composed);
            check(stop)?;
            return stream_pixels(stream, size);
        }
        let _staging = crate::pixels::reserve(n).ok_or_else(budget)?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(n).map_err(|_| budget())?;
        pixels.resize(n, 0);
        unsafe {
            composed
                .bitmap
                .CopyPixels(std::ptr::null(), w * 4, &mut pixels)?;
        }
        drop(composed);
        match fmt {
            Format::Jpeg(q) => {
                for i in 0..n / 4 {
                    if i % 65536 == 0 {
                        check(stop)?;
                    }
                    let white = 255 - pixels[4 * i + 3];
                    for j in 0..3 {
                        pixels[3 * i + j] = pixels[4 * i + j].saturating_add(white);
                    }
                }
                let (stream, size) = unsafe { jpeg(&wic, w, h, q, &pixels[..n / 4 * 3])? };
                check(stop)?;
                stream_pixels(stream, size)
            }
            Format::WebpLossless | Format::WebpLossy(_) => {
                for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
                    if i % 65536 == 0 {
                        check(stop)?;
                    }
                    let a = u32::from(p[3]);
                    for c in &mut p[..3] {
                        *c = (u32::from(*c) * 255 + a / 2)
                            .checked_div(a)
                            .unwrap_or(0)
                            .min(255) as u8;
                    }
                    p.swap(0, 2);
                }
                if let Format::WebpLossy(q) = fmt {
                    check(stop)?;
                    let mut opts = tiny_webp::Options::default();
                    opts.quality = q;
                    let bytes = tiny_webp::encode_rgba(&pixels, w, h, &opts)
                        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                    check(stop)?;
                    let out = encoding.split(bytes.capacity()).ok_or_else(budget)?;
                    return Ok(Pixels::from_reserved(bytes, out));
                }
                let lease = crate::pixels::reserve(0).ok_or_else(budget)?;
                let mut out = Sink {
                    bytes: Vec::new(),
                    lease,
                    stop,
                };
                image_webp::WebPEncoder::new(&mut out)
                    .encode(&pixels, w, h, image_webp::ColorType::Rgba8)
                    .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                check(stop)?;
                Ok(Pixels::from_reserved(out.bytes, out.lease))
            }
            Format::Png => unreachable!(),
        }
    }
}
struct Sink<'a> {
    bytes: Vec<u8>,
    lease: crate::byte_budget::Lease,
    stop: &'a AtomicBool,
}
impl io::Write for Sink<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.stop.load(Ordering::Acquire) {
            return Err(io::Error::other("Export cancelled"));
        }
        let size = self
            .bytes
            .len()
            .checked_add(buf.len())
            .filter(|n| *n <= 257 * 1024 * 1024)
            .ok_or_else(|| io::Error::other("WebP output too large"))?;
        if !self.lease.grow_to(size) {
            return Err(io::Error::other(budget()));
        }
        self.bytes
            .try_reserve_exact(buf.len())
            .map_err(io::Error::other)?;
        if !self.lease.grow_to(self.bytes.capacity()) {
            return Err(io::Error::other(budget()));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
unsafe fn jpeg(
    wic: &IWICImagingFactory,
    w: u32,
    h: u32,
    q: u8,
    pixels: &[u8],
) -> Result<(IStream, u64)> {
    use windows::{
        core::PWSTR,
        Win32::System::{
            Com::StructuredStorage::PROPBAG2,
            Variant::{VARIANT, VT_R4},
        },
    };
    let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
    let enc = wic.CreateEncoder(&GUID_ContainerFormatJpeg, std::ptr::null())?;
    enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
    let (mut frame, mut bag) = (None, None);
    enc.CreateNewFrame(&mut frame, &mut bag)?;
    let bag = bag.ok_or_else(|| Error::new(E_FAIL, "JPEG encoder has no quality control"))?;
    let mut name: Vec<u16> = "ImageQuality\0".encode_utf16().collect();
    let prop = PROPBAG2 {
        pstrName: PWSTR(name.as_mut_ptr()),
        ..Default::default()
    };
    let mut quality = VARIANT::default();
    (*quality.Anonymous.Anonymous).vt = VT_R4;
    (*quality.Anonymous.Anonymous).Anonymous.fltVal = f32::from(q) / 100.;
    bag.Write(1, &prop, &quality)?;
    let frame = frame.ok_or_else(|| Error::new(E_FAIL, "Missing JPEG frame"))?;
    frame.Initialize(&bag)?;
    frame.SetSize(w, h)?;
    let mut format = GUID_WICPixelFormat24bppBGR;
    frame.SetPixelFormat(&mut format)?;
    if format != GUID_WICPixelFormat24bppBGR {
        return Err(Error::new(E_FAIL, "Unsupported JPEG pixel format"));
    }
    frame.WritePixels(h, w * 3, pixels)?;
    frame.Commit()?;
    enc.Commit()?;
    let mut stat = STATSTG::default();
    stream.Stat(&mut stat, STATFLAG_NONAME)?;
    if !(1..=257 * 1024 * 1024).contains(&stat.cbSize) {
        return Err(budget());
    }
    stream.Seek(0, STREAM_SEEK_SET, None)?;
    Ok((stream, stat.cbSize))
}

#[cfg(test)]
#[path = "export_codec_tests.rs"]
mod tests;
