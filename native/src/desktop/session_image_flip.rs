//! 主干将底图翻转保存在标志里，栅格标注已物化翻转；这里只处理底图/预览。
use super::*;
use crate::{raster_flip, session_window::Window};
use serde::Deserialize;
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Flags {
    #[serde(default)]
    flipped_x: Option<bool>,
    #[serde(default)]
    flipped_y: Option<bool>,
}
impl Flags {
    pub(super) fn read(window: &Window<'_>) -> Result<Self> {
        window
            .image_edit_state
            .map(|raw| {
                serde_json::from_str(raw.get())
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored image flip flags"))
            })
            .transpose()
            .map(Option::unwrap_or_default)
    }
}
#[derive(Default)]
pub(super) struct Cache<'a>(HashMap<(&'a str, bool, bool), Image>);
impl<'a> Cache<'a> {
    pub(super) fn apply(
        &mut self,
        reference: Option<&'a str>,
        source: Option<Image>,
        flags: Flags,
        cancel: &AtomicBool,
    ) -> Result<Option<Image>> {
        check(cancel)?;
        let (Some(reference), Some(source)) = (reference, source) else {
            return Ok(None);
        };
        let x = flags.flipped_x.unwrap_or(false);
        let y = flags.flipped_y.unwrap_or(false);
        if !x && !y {
            return Ok(Some(source));
        }
        let key = (reference, x, y);
        if let Some(cached) = self.0.get(&key) {
            return Ok(Some(cached.clone()));
        }
        let pixels =
            raster_flip::image_axes(&source.pixels, (source.width, source.height), x, y, || {
                cancel.load(Ordering::Acquire)
            })
            .map_err(|error| {
                let code = match error {
                    raster_flip::Error::Cancelled => E_ABORT,
                    raster_flip::Error::BudgetExceeded => E_OUTOFMEMORY,
                    _ => E_INVALIDARG,
                };
                Error::new(code, "Cannot prepare restored flipped image")
            })?;
        check(cancel)?;
        let result = Image {
            width: source.width,
            height: source.height,
            pixels,
        };
        self.0
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot cache restored flip"))?;
        self.0.insert(key, result.clone());
        Ok(Some(result))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_reference_and_orientation_share_pixels_without_mutating_source() {
        let source = Image {
            width: 2,
            height: 2,
            pixels: vec![1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 255].into(),
        };
        let flags = Flags {
            flipped_x: Some(true),
            flipped_y: Some(true),
        };
        let cancel = AtomicBool::new(false);
        let mut cache = Cache::default();
        let first = cache
            .apply(Some("same"), Some(source.clone()), flags, &cancel)
            .unwrap()
            .unwrap();
        let second = cache
            .apply(Some("same"), Some(source.clone()), flags, &cancel)
            .unwrap()
            .unwrap();
        assert!(first.pixels.same_storage(&second.pixels));
        assert_eq!(
            &first.pixels[..],
            &[4, 4, 4, 255, 3, 3, 3, 255, 2, 2, 2, 255, 1, 1, 1, 255]
        );
        assert_eq!(source.pixels[0], 1);
        let unchanged = cache
            .apply(
                Some("same"),
                Some(source.clone()),
                Flags::default(),
                &cancel,
            )
            .unwrap()
            .unwrap();
        assert!(unchanged.pixels.same_storage(&source.pixels));
        cancel.store(true, Ordering::Release);
        assert!(cache
            .apply(Some("same"), Some(source), flags, &cancel)
            .is_err());
    }
    #[test]
    fn flags_are_read_even_without_crop_fields() {
        let window: Window<'_> = serde_json::from_str(
            r#"{"id":"u","x":0,"y":0,"w":1,"h":1,"imageEditState":{"flippedX":true}}"#,
        )
        .unwrap();
        let flags = Flags::read(&window).unwrap();
        assert_eq!(flags.flipped_x, Some(true));
        assert_eq!(flags.flipped_y, None);
    }
}
