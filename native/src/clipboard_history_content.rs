//! Accept only Bitmap/Text; separate encoding/pixel limits and preserve original encoding for the existing image-import thread.
use super::*;
use windows::{
    ApplicationModel::DataTransfer::{DataPackageView, StandardDataFormats},
    Storage::Streams::DataReader,
    Win32::Foundation::E_OUTOFMEMORY,
};

pub(super) fn decode(
    view: &DataPackageView,
    remaining: usize,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<Option<Content>> {
    check(cancel, deadline)?;
    // Prefer Bitmap; invalid Bitmap does not silently become the same item's Text or fall back to HTML/file lists.
    if view.Contains(&StandardDataFormats::Bitmap()?)? {
        let bitmap = wait::operation!(view.GetBitmapAsync()?, cancel, deadline)?;
        let stream = wait::operation!(bitmap.OpenReadAsync()?, cancel, deadline)?;
        let size = stream.Size()?;
        if size == 0 || size > MAX_IMAGE_BYTES.min(remaining) as u64 {
            return Ok(None);
        }
        let lease = crate::pixels::reserve(size as usize)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Clipboard history input budget exhausted"))?;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        let loaded = wait::operation!(reader.LoadAsync(size as u32)?, cancel, deadline)?;
        if loaded != size as u32 {
            return Ok(None);
        }
        let mut bytes = vec![0; size as usize];
        reader.ReadBytes(&mut bytes)?;
        let image = decode_bitmap(&bytes, cancel, deadline)?;
        let thumbnail = crate::image_thumbnail::Thumbnail::from_image(&image)?;
        let preview = super::preview::image(&image)?;
        check(cancel, deadline)?;
        if size as usize + thumbnail.pixels().len() + preview.pixels.len() > remaining {
            return Ok(None);
        }
        return Ok(Some(Content::Bitmap {
            preview,
            thumbnail,
            width: image.width,
            height: image.height,
            encoded: crate::pixels::Pixels::from_reserved(bytes, lease),
        }));
    }
    if view.Contains(&StandardDataFormats::Text()?)? {
        let value = wait::operation!(view.GetTextAsync()?, cancel, deadline)?;
        if value.len() > MAX_TEXT_BYTES.min(remaining) {
            return Ok(None);
        }
        let text = value.to_string();
        if text.len() > MAX_TEXT_BYTES.min(remaining)
            || text.trim().is_empty()
            || text.contains('\0')
        {
            return Ok(None);
        }
        let lines = text.replace("\r\n", "\n").replace('\r', "\n");
        if lines.split('\n').count() > 256 {
            return Ok(None);
        }
        check(cancel, deadline)?;
        return Ok(Some(Content::Text(text)));
    }
    Ok(None)
}

// Menu close and read deadlines reach WIC's pixel-allocation boundary; preserve history-read E_ABORT semantics.
pub(super) fn decode_bitmap(
    bytes: &[u8],
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<crate::image_decode::Image> {
    check(cancel, deadline)?;
    let image = crate::image_decode::Image::from_clipboard_history(bytes, &|| {
        cancel.load(Ordering::Acquire) || Instant::now() >= deadline
    });
    check(cancel, deadline)?;
    image
}
