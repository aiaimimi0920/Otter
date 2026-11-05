//! 将 MF 存储 surface 和可见尺寸分开；只接受原合同的右/下宏块 padding。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_INVALIDARG, Media::MediaFoundation::*},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub visible_width: u32,
    pub visible_height: u32,
}
impl Layout {
    pub(super) fn read(media: &IMFMediaType, width: u32, height: u32) -> Result<Self> {
        unsafe {
            let size = media.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let (stored_width, stored_height) = ((size >> 32) as u32, size as u32);
            let mut has_aperture = false;
            for attribute in [MF_MT_MINIMUM_DISPLAY_APERTURE, MF_MT_GEOMETRIC_APERTURE] {
                match media.GetBlobSize(&attribute) {
                    Ok(16) => {
                        let mut bytes = [0; 16];
                        media.GetBlob(&attribute, &mut bytes, None)?;
                        if !aperture(&bytes, width, height) {
                            return Err(invalid("Decoder visible aperture mismatch"));
                        }
                        has_aperture = true;
                    }
                    Ok(_) => return Err(invalid("Decoder aperture size invalid")),
                    Err(error) if error.code() == MF_E_ATTRIBUTENOTFOUND => {}
                    Err(error) => return Err(error),
                }
            }
            if !valid_surface(stored_width, stored_height, width, height, has_aperture) {
                return Err(invalid("Decoder negotiated surface mismatch"));
            }
            Ok(Self {
                width: stored_width,
                height: stored_height,
                visible_width: width,
                visible_height: height,
            })
        }
    }
}
fn valid_surface(sw: u32, sh: u32, width: u32, height: u32, aperture: bool) -> bool {
    // 同时校验调用方尺寸，避免未来 stream-change 接线绕过初始 configure 的边界。
    (2..=4096).contains(&width)
        && (2..=4096).contains(&height)
        && width.is_multiple_of(2)
        && height.is_multiple_of(2)
        && (sw == width || sw == width.div_ceil(16) * 16)
        && (sh == height || sh == height.div_ceil(16) * 16)
        && u64::from(sw) * u64::from(sh) <= 8_294_400
        && (aperture || (sw == width && sh == height))
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn aperture(bytes: &[u8; 16], width: u32, height: u32) -> bool {
    bytes[..8] == [0; 8]
        && i32::from_le_bytes(bytes[8..12].try_into().unwrap()) == width as i32
        && i32::from_le_bytes(bytes[12..].try_into().unwrap()) == height as i32
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_padding_requires_aperture_and_bounded_dimensions() {
        assert!(valid_surface(680, 430, 680, 430, false));
        assert!(valid_surface(688, 432, 680, 430, true));
        for (sw, sh, w, h, aperture) in [
            (688, 432, 680, 430, false),
            (704, 432, 680, 430, true),
            (672, 432, 680, 430, true),
            (688, 448, 680, 430, true),
            (4096, 4096, 4096, 4096, true),
            (0, 0, 0, 0, true),
            (16, 16, 3, 2, true),
            (16, 16, u32::MAX, 2, true),
        ] {
            assert!(!valid_surface(sw, sh, w, h, aperture));
        }
    }
    #[test]
    fn aperture_rejects_nonzero_offsets_and_wrong_visible_extent() {
        let mut bytes = [0; 16];
        bytes[8..12].copy_from_slice(&680_i32.to_le_bytes());
        bytes[12..].copy_from_slice(&430_i32.to_le_bytes());
        assert!(aperture(&bytes, 680, 430));
        assert!(!aperture(&bytes, 688, 432));
        for offset in 0..8 {
            let mut bad = bytes;
            bad[offset] = 1;
            assert!(!aperture(&bad, 680, 430));
        }
    }
}
