//! Background loading of session inline images; borrow Data URLs and reuse shared pixel budgets and the WIC decoder owner.
use super::image::Image;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

/// Accept inline Data URLs only; paths/network resources require separate authorization, never Base64 fallback.
/// Callers must run this in the background; cancellation checks cannot forcibly interrupt an executing native WIC call.
pub fn inline(reference: &str, cancel: &AtomicBool) -> Result<Image> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Session image cancelled"));
    }
    let (mime, encoded) = reference
        .strip_prefix("data:")
        .and_then(|value| value.split_once(";base64,"))
        .ok_or_else(|| {
            Error::new(
                E_INVALIDARG,
                "Session image is not an inline Base64 resource",
            )
        })?;
    super::loom_output_inline::decode_base64(mime, encoded, cancel)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_session_data_url_decodes_on_worker_and_releases_pixels() {
        let expected: crate::pixels::Pixels = vec![10, 20, 30, 255, 40, 50, 60, 255].into();
        let encoded = super::super::loom_inline::prepare(
            2,
            1,
            expected.clone(),
            &crate::editor::Editor::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let crate::loom_art_wire::Input::InlinePng { base64, .. } = encoded.descriptor() else {
            panic!("inline PNG expected");
        };
        let bytes = serde_json::to_vec(
            &serde_json::json!({"stickers":[{"id":"s","x":0,"y":0,"w":200,"h":100,
            "src":format!("data:image/png;base64,{base64}")}],"links":[]}),
        )
        .unwrap();
        let image = std::thread::spawn(move || {
            let document = crate::session_document::decode(&bytes).unwrap();
            let references = crate::session_assets::References::from_window(&document.windows[0]);
            inline(references.source.unwrap(), &AtomicBool::new(false)).unwrap()
        })
        .join()
        .unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.pixels, expected);
        let weak = image.pixels.downgrade();
        drop(image);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn paths_network_and_non_images_are_not_implicitly_loaded() {
        for reference in [
            "C:\\image.png",
            "https://example.invalid/image.png",
            "file:///C:/image.png",
            "data:text/plain;base64,YQ==",
            "data:image/png;base64,",
            "data:image/png;base64,invalid",
        ] {
            assert!(inline(reference, &AtomicBool::new(false)).is_err());
        }
        assert_eq!(
            inline("invalid", &AtomicBool::new(true))
                .err()
                .unwrap()
                .code(),
            E_ABORT
        );
    }
}
