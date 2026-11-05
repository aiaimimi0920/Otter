//! 背景线程从实际上传PNG生成可信预览；复用WIC/Thumbnail/原像素预算，不信脚本预览。
use super::{
    super::{Call, Error},
    Preview,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use windows::Win32::{Foundation::RPC_E_CHANGED_MODE, System::Com::*};
struct Apartment(bool);
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
    let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if init != RPC_E_CHANGED_MODE {
        init.ok().map_err(|_| Error::Wire)?;
    }
    let _apartment = Apartment(init != RPC_E_CHANGED_MODE);
    let mut remaining = uploads.decoded_limit;
    let mut result = Vec::new();
    for (upload, (label, size)) in uploads.images.iter().zip(&uploads.sources) {
        if stopped() {
            return Err(Error::Stopped);
        }
        let _encoded = crate::pixels::reserve(upload.png.len()).ok_or(Error::Limit)?;
        let bytes = STANDARD.decode(&upload.png).map_err(|_| Error::Wire)?;
        let texture = crate::image_decode::shader::png(&bytes, remaining, stopped)
            .map_err(|_| Error::Wire)?;
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
