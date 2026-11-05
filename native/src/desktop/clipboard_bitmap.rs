//! 同一次合成的 PNG 与 DIBV5 表示；DIB 使用非预乘 BGRA，避免半透明颜色再次变暗。
use crate::pixels::Pixels;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Imaging::*},
};

pub(super) struct Payload {
    pub png: Pixels,
    pub dibv5: Pixels,
}
const HEADER: usize = 124;

fn header(width: u32, height: u32) -> Result<([u8; HEADER], usize)> {
    let count = crate::viewport::checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid clipboard bitmap dimensions"))?;
    let mut bytes = [0; HEADER];
    // 明确写入 little-endian 字段，不读取结构体 padding；负高度表示自顶向下。
    for (offset, value) in [
        (0, HEADER as u32),
        (4, width),
        (8, (-(height as i32)) as u32),
        (16, 3),
        (20, count as u32), // BI_BITFIELDS
        (40, 0x00ff0000),
        (44, 0x0000ff00),
        (48, 0x000000ff),
        (52, 0xff000000),
        (56, 0x73524742),
        (108, 4), // sRGB, LCS_GM_IMAGES
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
    Ok((bytes, count))
}

pub(super) fn encode(wic: &IWICImagingFactory, bitmap: &IWICBitmap) -> Result<Pixels> {
    unsafe {
        let (mut width, mut height) = (0, 0);
        bitmap.GetSize(&mut width, &mut height)?;
        let (header, count) = header(width, height)?;
        let mut lease = crate::pixels::reserve(HEADER + count)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "DIBV5 clipboard budget exhausted"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(HEADER + count)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate clipboard bitmap"))?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::new(E_OUTOFMEMORY, "DIBV5 capacity exceeds budget"));
        }
        bytes.resize(HEADER + count, 0);
        bytes[..HEADER].copy_from_slice(&header);
        let converter = wic.CreateFormatConverter()?;
        converter.Initialize(
            bitmap,
            &GUID_WICPixelFormat32bppBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        converter.CopyPixels(std::ptr::null(), width * 4, &mut bytes[HEADER..])?;
        Ok(Pixels::from_reserved(bytes, lease))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_has_masks_top_down_dimensions_and_no_profile_payload() {
        let (bytes, count) = header(3, 2).unwrap();
        assert_eq!(count, 24);
        assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 124);
        assert_eq!(i32::from_le_bytes(bytes[8..12].try_into().unwrap()), -2);
        assert_eq!(&bytes[12..16], &[1, 0, 32, 0]);
        assert_eq!(
            &bytes[40..56],
            &[0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255]
        );
        assert_eq!(&bytes[112..], &[0; 12]);
        assert!(header(0, 1).is_err());
        assert!(header(u32::MAX, u32::MAX).is_err());
    }
    #[test]
    fn conversion_unpremultiplies_colors_and_keeps_row_order_and_alpha() {
        use windows::Win32::System::Com::*;
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let bitmap = wic
                .CreateBitmapFromMemory(
                    1,
                    3,
                    &GUID_WICPixelFormat32bppPBGRA,
                    4,
                    &[64, 0, 0, 128, 0, 255, 0, 255, 0, 0, 0, 0],
                )
                .unwrap();
            let dib = encode(&wic, &bitmap).unwrap();
            assert_eq!(dib.len(), HEADER + 12);
            assert!((127..=128).contains(&dib[HEADER]));
            assert_eq!(&dib[HEADER + 1..], &[0, 0, 128, 0, 255, 0, 255, 0, 0, 0, 0]);
        }
    }
}
