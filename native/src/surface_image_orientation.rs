//! EXIF orientation for Surface JPEG/TIFF only; WIC source chains remain on the decoding thread, and pixels are materialized once.
use super::*;
use windows::{
    core::w,
    Win32::System::{
        Com::StructuredStorage::{PropVariantClear, PropVariantToUInt16, PROPVARIANT},
        Variant::VT_UI2,
    },
};

struct Metadata(PROPVARIANT);
impl Drop for Metadata {
    fn drop(&mut self) {
        // Clean up output even if metadata queries fail; metadata is not necessarily an allocation-free integer.
        unsafe {
            let _ = PropVariantClear(&mut self.0);
        }
    }
}
unsafe fn direction(frame: &IWICBitmapFrameDecode, mime: &str) -> u16 {
    let path = match mime {
        "image/jpeg" => w!("/app1/ifd/{ushort=274}"),
        "image/tiff" => w!("/ifd/{ushort=274}"),
        _ => return 1,
    };
    unsafe {
        let Ok(reader) = frame.GetMetadataQueryReader() else {
            return 1;
        };
        let mut value = Metadata(PROPVARIANT::default());
        if reader.GetMetadataByName(path, &mut value.0).is_err()
            || value.0.Anonymous.Anonymous.vt != VT_UI2
        {
            return 1;
        }
        let value = PropVariantToUInt16(&value.0).unwrap_or(1);
        if (1..=8).contains(&value) {
            value
        } else {
            1
        }
    }
}
fn active(stopped: &impl Fn() -> bool) -> Result<()> {
    if stopped() {
        Err(Error::new(E_INVALIDARG, "Surface orientation cancelled"))
    } else {
        Ok(())
    }
}
pub(super) unsafe fn source(
    factory: &IWICImagingFactory,
    decoder: &IWICBitmapDecoder,
    mime: &str,
    stopped: &impl Fn() -> bool,
) -> Result<IWICBitmapSource> {
    unsafe {
        let frame = decoder.GetFrame(0)?;
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height)?;
        // Check original dimensions before metadata queries and transform initialization to prevent decoder-internal caches from bypassing admission.
        checked_image_bytes(width, height)
            .filter(|n| *n <= MAX_PIXELS)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Image exceeds native decode budget"))?;
        active(stopped)?;
        let direction = direction(&frame, mime);
        let mut source: IWICBitmapSource = frame.cast()?;
        let transforms: &[WICBitmapTransformOptions] = match direction {
            2 => &[WICBitmapTransformFlipHorizontal],
            3 => &[WICBitmapTransformRotate180],
            4 => &[WICBitmapTransformFlipVertical],
            5 => &[WICBitmapTransformRotate90, WICBitmapTransformFlipHorizontal],
            6 => &[WICBitmapTransformRotate90],
            7 => &[WICBitmapTransformRotate90, WICBitmapTransformFlipVertical],
            8 => &[WICBitmapTransformRotate270],
            _ => &[],
        };
        // Two independent transforms make diagonal orientation order explicit without another full-image BGRA Vec.
        for transform in transforms {
            active(stopped)?;
            let rotator = factory.CreateBitmapFlipRotator()?;
            rotator.Initialize(&source, *transform)?;
            source = rotator.cast()?;
        }
        active(stopped)?;
        Ok(source)
    }
}
