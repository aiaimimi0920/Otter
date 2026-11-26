//! Sticker-upload-specific dimension contract; the 100MP limit changes neither other decode entries nor the shared 512MiB total pixel budget.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{
    WINCODEC_ERR_COMPONENTINITIALIZEFAILURE, WINCODEC_ERR_COMPONENTNOTFOUND,
};
impl Image {
    pub(crate) fn from_sticker(bytes: &[u8], cancel: &AtomicBool) -> Result<Self> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::new(
                windows::Win32::Foundation::E_ABORT,
                "Sticker decode cancelled",
            ));
        }
        sticker_result(bytes, cancel, Self::from_sticker_wic(bytes, cancel))
    }
    fn from_sticker_wic(bytes: &[u8], cancel: &AtomicBool) -> Result<Self> {
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
            let frame = decoder.GetFrame(0)?;
            let (mut width, mut height) = (0, 0);
            frame.GetSize(&mut width, &mut height)?;
            if width == 0
                || height == 0
                || width > 32768
                || height > 32768
                || u64::from(width) * u64::from(height) > 100_000_000
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Sticker image dimensions exceed policy",
                ));
            }
            Self::decode_source(
                &factory,
                &frame.cast()?,
                super::SizePolicy::Any,
                400_000_000,
                &|| cancel.load(Ordering::Acquire),
            )
        }
    }
}

fn sticker_result(bytes: &[u8], cancel: &AtomicBool, result: Result<Image>) -> Result<Image> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Sticker decode cancelled",
        ));
    }
    match result {
        Err(error)
            if matches!(
                error.code(),
                WINCODEC_ERR_COMPONENTNOTFOUND | WINCODEC_ERR_COMPONENTINITIALIZEFAILURE
            ) && bytes.len() <= 16 * 1024 * 1024
                && crate::surface_webp_header::inspect(bytes).is_ok() =>
        {
            // WIC can report a missing/uninitializable component on some installations.
            // Only those errors admit the existing restricted static WebP
            // decoder. Larger/animated images keep the original WIC contract/error;
            // neither Surface's Job/pixel limits nor Sticker's admission grows.
            let decoded =
                Image::from_surface(bytes, "image/webp", None, || cancel.load(Ordering::Acquire));
            if cancel.load(Ordering::Acquire) {
                return Err(Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Sticker decode cancelled",
                ));
            }
            decoded
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::WINCODEC_ERR_BADIMAGE;
    #[test]
    fn sticker_webp_missing_codec_uses_bounded_child_and_preserves_errors() {
        let bytes = Image::webp_for_test();
        let cancel = AtomicBool::new(false);
        for code in [
            WINCODEC_ERR_COMPONENTNOTFOUND,
            WINCODEC_ERR_COMPONENTINITIALIZEFAILURE,
        ] {
            let image = sticker_result(&bytes, &cancel, Err(Error::new(code, "codec unavailable")))
                .unwrap();
            assert_eq!((image.width, image.height), (2, 1));
            assert_eq!(&image.pixels[..], &[0, 0, 255, 255, 0, 128, 0, 128]);
        }
        for code in [
            WINCODEC_ERR_BADIMAGE,
            E_INVALIDARG,
            windows::Win32::Foundation::E_ABORT,
        ] {
            assert_eq!(
                sticker_result(&bytes, &cancel, Err(Error::new(code, "original failure")))
                    .err()
                    .unwrap()
                    .code(),
                code
            );
        }
        assert_eq!(
            sticker_result(
                b"not webp",
                &cancel,
                Err(Error::new(
                    WINCODEC_ERR_COMPONENTNOTFOUND,
                    "original failure"
                ))
            )
            .err()
            .unwrap()
            .code(),
            WINCODEC_ERR_COMPONENTNOTFOUND
        );
        // Unsupported animation and input sizes must preserve WIC's original error,
        // not enter the narrower Surface worker or expand its resource allowance.
        let mut animated = bytes.clone();
        animated.extend_from_slice(b"ANIM\0\0\0\0");
        let riff_len = (animated.len() - 8) as u32;
        animated[4..8].copy_from_slice(&riff_len.to_le_bytes());
        for unsupported in [animated, vec![0; 16 * 1024 * 1024 + 1]] {
            assert_eq!(
                sticker_result(
                    &unsupported,
                    &cancel,
                    Err(Error::new(
                        WINCODEC_ERR_COMPONENTNOTFOUND,
                        "original failure"
                    ))
                )
                .err()
                .unwrap()
                .code(),
                WINCODEC_ERR_COMPONENTNOTFOUND
            );
        }
        cancel.store(true, Ordering::Release);
        assert_eq!(
            Image::from_sticker(&bytes, &cancel).err().unwrap().code(),
            windows::Win32::Foundation::E_ABORT
        );
    }
}
