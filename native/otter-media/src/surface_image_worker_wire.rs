//! Image-child frame boundaries; validate only numbers, without parsing encoded images or allocating pixels.
use std::io;
pub const MAX_INPUT: usize = 16 * 1024 * 1024;
const MAX_PIXELS: usize = 64 * 1024 * 1024;
// Fixed codec/version; do not select decoders by guessing or passing command strings.
pub const REQUEST: &[u8; 4] = b"OWI1";
pub const RESPONSE: &[u8; 4] = b"OWP1";
#[derive(Clone, Copy, PartialEq)]
pub enum Codec {
    WebP,
    Svg,
}
impl Codec {
    pub fn request(self) -> &'static [u8; 4] {
        match self {
            Self::WebP => REQUEST,
            Self::Svg => b"OSI2",
        }
    }
    pub fn response(self) -> &'static [u8; 4] {
        match self {
            Self::WebP => RESPONSE,
            Self::Svg => b"OSP2",
        }
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid image worker frame")
}
// Validate SVG v2 dimension extensions independently; raster images always use one pixel per logical unit, rounded up.
pub fn logical_size(size: [f32; 2], raster: [u32; 2]) -> io::Result<[f32; 2]> {
    if size.iter().zip(raster).any(|(v, r)| {
        !v.is_finite()
            || *v <= 0.0
            || !(32768. / *v).is_finite()
            || *v > 16384.0
            || v.ceil() as u32 != r
    }) {
        return Err(invalid());
    }
    Ok(size)
}
pub fn pixel_count(width: u32, height: u32) -> io::Result<usize> {
    crate::viewport::checked_image_bytes(width, height)
        .filter(|count| *count <= MAX_PIXELS)
        .ok_or_else(invalid)
}
#[cfg(test)]
pub fn request(header: &[u8; 8]) -> io::Result<usize> {
    request_for(Codec::WebP, header)
}
pub fn request_for(codec: Codec, header: &[u8; 8]) -> io::Result<usize> {
    let length = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
    if &header[..4] != codec.request() || length == 0 || length > MAX_INPUT {
        return Err(invalid());
    }
    Ok(length)
}
#[cfg(any(test, feature = "test-support"))]
pub fn response(header: &[u8; 16]) -> io::Result<(u32, u32, usize)> {
    response_for(Codec::WebP, header)
}
pub fn response_for(codec: Codec, header: &[u8; 16]) -> io::Result<(u32, u32, usize)> {
    if &header[..4] != codec.response() {
        return Err(invalid());
    }
    let width = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let height = u32::from_le_bytes(header[8..12].try_into().unwrap());
    let claimed = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
    let count = pixel_count(width, height)?;
    if claimed != count {
        return Err(invalid());
    }
    Ok((width, height, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_worker_codec_frames_cannot_cross_decode() {
        let mut request = *b"OSI2\x01\0\0\0";
        assert_eq!(request_for(Codec::Svg, &request).unwrap(), 1);
        assert!(request_for(Codec::WebP, &request).is_err());
        request[..4].copy_from_slice(REQUEST);
        assert!(request_for(Codec::Svg, &request).is_err());
        let header = *b"OSP2\x01\0\0\0\x01\0\0\0\x04\0\0\0";
        assert_eq!(response_for(Codec::Svg, &header).unwrap(), (1, 1, 4));
        assert!(response_for(Codec::WebP, &header).is_err());
    }
    #[test]
    fn image_worker_request_checks_type_and_length_without_allocation() {
        for (length, valid) in [
            (0, false),
            (1, true),
            (MAX_INPUT as u32, true),
            (MAX_INPUT as u32 + 1, false),
            (u32::MAX, false),
        ] {
            let mut header = [0; 8];
            header[..4].copy_from_slice(REQUEST);
            header[4..].copy_from_slice(&length.to_le_bytes());
            assert_eq!(request(&header).is_ok(), valid);
            header[0] = b'X';
            assert!(request(&header).is_err());
        }
    }
    #[test]
    fn image_worker_frame_rejects_untrusted_dimensions_before_allocation() {
        for (width, height, count, valid) in [
            (2, 1, 8, true),
            (0, 1, 0, false),
            (1, 0, 0, false),
            (u32::MAX, u32::MAX, 4, false),
            (4096, 4096, 64 * 1024 * 1024, true),
            (4096, 4097, 4096 * 4097 * 4, false),
            (2, 1, 4, false),
            (2, 1, 12, false),
        ] {
            let mut header = [0; 16];
            header[..4].copy_from_slice(RESPONSE);
            for (slot, value) in header[4..].chunks_exact_mut(4).zip([width, height, count]) {
                slot.copy_from_slice(&value.to_le_bytes());
            }
            assert_eq!(response(&header).is_ok(), valid);
            header[0] = b'X';
            assert!(response(&header).is_err());
        }
    }
}
