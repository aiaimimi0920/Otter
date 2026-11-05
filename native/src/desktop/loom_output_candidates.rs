//! 后台准备有界候选卡片；不联网、不读本地路径，也不把预览当正式端口。
use super::*;
use crate::image_thumbnail::Thumbnail;
use std::sync::Arc;

pub(crate) struct Item {
    pub index: u32,
    pub title: String,
    pub thumbnail: Option<Thumbnail>,
}
pub(crate) struct Set {
    pub items: Vec<Item>,
    pub selected: Option<u32>,
}
impl Set {
    pub(super) fn prepare(
        value: Option<&Value>,
        cancel: &AtomicBool,
    ) -> Result<Option<Arc<Self>>, Error> {
        let Some(value) = value else { return Ok(None) };
        let Some(items) = value.get("items").and_then(Value::as_array) else {
            return Ok(None);
        };
        if items.len() > 64 {
            return Err(Error::InvalidCandidates);
        }
        let mut prepared = Vec::<Item>::new();
        prepared
            .try_reserve_exact(items.len())
            .map_err(|_| Error::Allocation)?;
        let mut remaining = 16 * 1024 * 1024;
        for item in items {
            if cancel.load(Ordering::Acquire) {
                return Err(Error::Cancelled);
            }
            let Some(index) = item
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n < i32::MAX as u32)
            else {
                continue;
            };
            if prepared.iter().any(|item| item.index == index) {
                return Err(Error::InvalidCandidates);
            }
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .take(128)
                .collect();
            let thumbnail = ["thumbnail", "preview", "thumbnailUrl", "imageUrl"]
                .into_iter()
                .filter_map(|key| item.get(key).and_then(Value::as_str))
                .find_map(|uri| preview(uri, cancel, &mut remaining));
            prepared.push(Item {
                index,
                title,
                thumbnail,
            });
        }
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let selected = value
            .get("selectedIndex")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| prepared.iter().any(|item| item.index == *n));
        Ok((prepared.len() > 1).then(|| {
            Arc::new(Self {
                items: prepared,
                selected,
            })
        }))
    }
}
fn preview(uri: &str, cancel: &AtomicBool, remaining: &mut usize) -> Option<Thumbnail> {
    if uri.len() > (4 * 1024 * 1024usize).div_ceil(3) * 4 + 64 {
        return None;
    }
    let (header, encoded) = uri.split_once(',')?;
    let mime = header.strip_prefix("data:")?.strip_suffix(";base64")?;
    if !matches!(mime, "image/png" | "image/jpeg" | "image/gif" | "image/bmp")
        || encoded.len() > (4 * 1024 * 1024usize).div_ceil(3) * 4
        || encoded.len() > *remaining
    {
        return None;
    }
    *remaining -= encoded.len();
    let bytes = loom_output_inline::decode_bytes(mime, encoded, cancel).ok()?;
    let _com = crate::desktop::Apartment::new().ok()?;
    let image = Image::from_candidate_thumbnail(&bytes, &|| cancel.load(Ordering::Acquire)).ok()?;
    Thumbnail::from_image(&image).ok()
}

#[cfg(test)]
#[path = "loom_output_candidates_tests.rs"]
mod tests;
