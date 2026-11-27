//! Decode one admitted input on the open worker. Carry transient region provenance until UI commit.
use super::*;
pub(super) fn load_input(
    input: Input,
    pinned: bool,
    cancel: &AtomicBool,
) -> std::result::Result<Option<PreparedDocument>, String> {
    let stopped = || cancel.load(Ordering::Acquire);
    if stopped() {
        return Ok(None);
    }
    let _apartment = Apartment::new().map_err(|e| e.to_string())?;
    use crate::desktop::image::Image;
    let prepare = |image| PreparedDocument::from_image_with_cancel(image, pinned, stopped);
    let prepared = match input {
        Input::TextCard(units) => crate::desktop::text_pin_builder::prepare(units, stopped),
        Input::Ordinary(captured, origin, region, meta) => {
            PreparedDocument::from_capture_with_cancel(
                captured,
                pinned,
                Some(origin),
                None,
                stopped,
            )
            .map(|mut p| {
                p.ordinary_region = region;
                p.ordinary_capture = true;
                p.capture_source
                    .as_mut()
                    .expect("ordinary capture provenance")
                    .naming = meta;
                p
            })
        }
        Input::WithContext(input) => {
            let (captured, origin, context) = *input;
            PreparedDocument::from_capture_with_cancel(
                captured,
                pinned,
                origin,
                Some(context),
                stopped,
            )
        }
        Input::Files(_) => return Err("Batch input requires batch preparation".into()),
        Input::Image(image) => prepare(image),
        Input::Editable(content, placement) => content.prepare(stopped).map(|mut p| {
            p.placement = placement;
            p
        }),
        Input::Captured(captured, origin) => {
            PreparedDocument::from_capture_with_cancel(captured, pinned, origin, None, stopped)
        }
        Input::File(path) => Image::open_with_cancel(&path, &stopped).and_then(prepare),
        Input::Encoded(bytes) => {
            Image::from_encoded_with_cancel(&bytes, &stopped).and_then(prepare)
        }
        Input::Dib(bytes) => {
            crate::desktop::clipboard_dib::decode_with_cancel(&bytes, stopped).and_then(prepare)
        }
    };
    if stopped() {
        Ok(None)
    } else {
        prepared.map(Some).map_err(|e| e.to_string())
    }
}
