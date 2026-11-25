//! Surface static bitmap and raw RGBA decoding; share the Pixels budget without temporary files or cross-thread COM.
use super::*;
#[path = "surface_image_orientation.rs"]
mod orientation;
#[path = "surface_webp_decode.rs"]
mod webp;
#[path = "surface_image_worker_client.rs"]
mod worker_client;
const MAX_PIXELS: usize = 64 * 1024 * 1024;
impl Image {
    /// Surface installation retains logical dimensions; generic Image carries only the raster and cannot determine SVG layout.
    pub(crate) fn from_surface_sized(
        bytes: &[u8],
        mime: &str,
        dimensions: Option<(u32, u32)>,
        stopped: impl Fn() -> bool,
    ) -> Result<(Self, [f32; 2])> {
        Self::from_surface_sized_scoped(None, bytes, mime, dimensions, stopped)
    }
    pub(crate) fn from_surface_sized_scoped(
        scope: Option<&str>,
        bytes: &[u8],
        mime: &str,
        dimensions: Option<(u32, u32)>,
        stopped: impl Fn() -> bool,
    ) -> Result<(Self, [f32; 2])> {
        // Pool-key capacity is only an optimization admission gate and must not narrow the original inline-image decoding contract.
        let scope =
            scope.filter(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control));
        if crate::surface_image_format::svg(mime) {
            if let Some(origin) = scope {
                return crate::surface_image_worker_session::decode_scoped(
                    origin,
                    bytes,
                    crate::surface_image_worker_wire::Codec::Svg,
                    None,
                    &stopped,
                )
                .map_err(|e| Error::new(E_INVALIDARG, format!("Surface SVG decode: {e}")));
            }
            return worker_client::run_sized(
                bytes,
                crate::surface_image_worker_wire::Codec::Svg,
                None,
                &stopped,
                |endpoint, deadline| {
                    crate::surface_image_worker_process::spawn_svg(endpoint, deadline, &stopped)
                },
            )
            .map_err(|e| Error::new(E_INVALIDARG, format!("Surface SVG decode: {e}")));
        }
        if let Some(origin) = scope {
            if bytes.len() <= 16 * 1024 * 1024
                && !bytes.is_empty()
                && mime != "application/x-neuro-rgba8"
                && crate::surface_image_format::encoded_mime(bytes, mime) == Some("image/webp")
            {
                let (width, height) = crate::surface_webp_header::inspect(bytes)
                    .map_err(|e| Error::new(E_INVALIDARG, format!("Surface WebP decode: {e}")))?;
                return crate::surface_image_worker_session::decode_scoped(
                    origin,
                    bytes,
                    crate::surface_image_worker_wire::Codec::WebP,
                    Some((width, height)),
                    &stopped,
                )
                .map_err(|e| Error::new(E_INVALIDARG, format!("Surface WebP decode: {e}")));
            }
        }
        Self::from_surface(bytes, mime, dimensions, stopped).map(|image| {
            let size = [image.width as f32, image.height as f32];
            (image, size)
        })
    }
    pub(crate) fn from_surface(
        bytes: &[u8],
        mime: &str,
        dimensions: Option<(u32, u32)>,
        stopped: impl Fn() -> bool,
    ) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 || stopped() {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid or cancelled Surface image",
            ));
        }
        if mime == "application/x-neuro-rgba8" {
            return rgba(bytes, dimensions, stopped);
        }
        if crate::surface_image_format::svg(mime) {
            return Self::from_surface_sized(bytes, mime, dimensions, stopped)
                .map(|(image, _)| image);
        }
        // Ordinary Surface containers are determined by verified bytes, not Tile/Wall's strict declared-MIME matching contract.
        let mime = crate::surface_image_format::encoded_mime(bytes, mime)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Unsupported Surface image representation"))?;
        if mime == "image/webp" {
            return webp::decode(bytes, &stopped);
        }
        // Actual animation still requires a separate contract; never silently take only the first frame.
        let container = match mime {
            "image/png" => GUID_ContainerFormatPng,
            "image/jpeg" => GUID_ContainerFormatJpeg,
            "image/bmp" => GUID_ContainerFormatBmp,
            "image/gif" => GUID_ContainerFormatGif,
            "image/tiff" => GUID_ContainerFormatTiff,
            "image/x-icon" | "image/vnd.microsoft.icon" => GUID_ContainerFormatIco,
            _ => return Err(Error::new(E_INVALIDARG, "Unsupported Surface image MIME")),
        };
        if mime == "image/png" && animated_png(bytes)? {
            return Err(Error::new(
                E_INVALIDARG,
                "Animated Surface PNG is not implemented",
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
            if decoder.GetContainerFormat()? != container || decoder.GetFrameCount()? != 1 {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Surface image container or frame count unsupported",
                ));
            }
            // Declared encoded-image dimensions are not authoritative natural dimensions; use WIC frame dimensions and bound them before allocation.
            let source = orientation::source(&factory, &decoder, mime, &stopped)?;
            let image = Self::decode_source(
                &factory,
                &source,
                super::SizePolicy::Any,
                MAX_PIXELS,
                &stopped,
            )?;
            if stopped() {
                return Err(Error::new(E_INVALIDARG, "Surface image decode cancelled"));
            }
            Ok(image)
        }
    }
}
fn rgba(bytes: &[u8], dimensions: Option<(u32, u32)>, stopped: impl Fn() -> bool) -> Result<Image> {
    let (width, height) =
        dimensions.ok_or_else(|| Error::new(E_INVALIDARG, "Missing RGBA dimensions"))?;
    let count = checked_image_bytes(width, height)
        .filter(|n| *n <= MAX_PIXELS && *n == bytes.len())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid RGBA dimensions"))?;
    let (mut pixels, lease) = pixel_buffer(count)?;
    for (input, output) in bytes
        .chunks(width as usize * 4)
        .zip(pixels.chunks_mut(width as usize * 4))
    {
        if stopped() {
            return Err(Error::new(E_INVALIDARG, "Surface RGBA decode cancelled"));
        }
        for (src, dst) in input.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
            let alpha = u16::from(src[3]);
            for (to, from) in [(0, 2), (1, 1), (2, 0)] {
                dst[to] = ((u16::from(src[from]) * alpha + 127) / 255) as u8;
            }
            dst[3] = src[3];
        }
    }
    Ok(Image {
        width,
        height,
        pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
    })
}
pub(super) fn animated_png(bytes: &[u8]) -> Result<bool> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::new(E_INVALIDARG, "Invalid PNG signature"));
    }
    let mut offset = 8usize;
    while offset < bytes.len() {
        let chunk = bytes
            .get(offset..offset + 8)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Truncated PNG chunk"))?;
        let size = u32::from_be_bytes(chunk[..4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12)
            .and_then(|n| n.checked_add(size))
            .filter(|n| *n <= bytes.len())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid PNG chunk size"))?;
        if &chunk[4..8] == b"acTL" {
            return Ok(true);
        }
        if &chunk[4..8] == b"IEND" {
            break;
        }
        offset = end;
    }
    Ok(false)
}
#[cfg(test)]
#[path = "surface_image_decode_tests.rs"]
mod tests;
