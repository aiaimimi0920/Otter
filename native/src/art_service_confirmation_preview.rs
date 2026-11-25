//! 背景线程从实际上传PNG生成可信预览；复用WIC/Thumbnail/原像素预算，不信脚本预览。
use super::{
    super::{Call, Error},
    Preview,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use windows::Win32::{Foundation::RPC_E_CHANGED_MODE, System::Com::*};
struct Apartment(bool);
impl Apartment {
    fn new() -> Result<Self, Error> {
        let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if init != RPC_E_CHANGED_MODE {
            init.ok().map_err(|_| Error::Wire)?;
        }
        Ok(Self(init != RPC_E_CHANGED_MODE))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
pub(crate) fn prepare(call: &Call<'_>, stopped: &impl Fn() -> bool) -> Result<Vec<Preview>, Error> {
    let Some(uploads) = &call.images else {
        return Ok(Vec::new());
    };
    let _apartment = Apartment::new()?;
    let mut remaining = uploads.decoded_limit;
    let mut result = Vec::new();
    for (upload, (label, size)) in uploads.images.iter().zip(&uploads.sources) {
        if stopped() {
            return Err(Error::Stopped);
        }
        let _encoded = crate::pixels::reserve(upload.png.len()).ok_or(Error::Limit)?;
        let bytes = STANDARD.decode(&upload.png).map_err(|_| Error::Wire)?;
        let texture = decode_png(&bytes, remaining, stopped)?;
        if [texture.width, texture.height] != *size {
            return Err(Error::Identity);
        }
        remaining = remaining
            .checked_sub(texture.rgba.len())
            .ok_or(Error::Limit)?;
        let image = crate::image_thumbnail::Thumbnail::from_rgba(
            texture.width,
            texture.height,
            &texture.rgba,
        )
        .map_err(|_| Error::Limit)?;
        result.push(Preview {
            label: label.clone(),
            size: *size,
            image,
        });
    }
    if stopped() || result.len() != uploads.images.len() {
        return Err(Error::Stopped);
    }
    Ok(result)
}

fn decode_png(
    bytes: &[u8],
    remaining: usize,
    stopped: &impl Fn() -> bool,
) -> Result<crate::image_decode::shader::Texture, Error> {
    // WIC 的 HRESULT 不区分取消和坏图；以解码器实际观察到的停止为准。
    let cancelled = std::cell::Cell::new(false);
    crate::image_decode::shader::png(bytes, remaining, &|| {
        let stop = stopped();
        if stop {
            cancelled.set(true);
        }
        stop
    })
    .map_err(|_| {
        if cancelled.get() {
            Error::Stopped
        } else {
            Error::Wire
        }
    })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
mod tests {
    use super::*;

    #[test]
    fn art_service_confirmation_preview_distinguishes_decode_cancellation_from_bad_png() {
        let _apartment = Apartment::new().unwrap();
        let image = crate::surface_javascript_wire::images::tests::resource();
        let checks = std::cell::Cell::new(0);
        let stopped = || {
            checks.set(checks.get() + 1);
            checks.get() >= 2
        };
        assert!(matches!(
            decode_png(&image.png, 16, &stopped),
            Err(Error::Stopped)
        ));
        assert_eq!(checks.get(), 2);
        assert!(matches!(
            decode_png(&image.png[..8], 16, &|| false),
            Err(Error::Wire)
        ));
        let decoded = decode_png(&image.png, 16, &|| false).unwrap();
        assert_eq!([decoded.width, decoded.height], [2, 2]);
    }
}
