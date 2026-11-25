//! 将已验证窗口字段与图像尺寸装配为恢复位置，不取得工作区或原文所有权。
use super::*;

pub(super) fn prepare(
    document: &session_document::Document<'_>,
    images: &[Images],
    cancel: &AtomicBool,
) -> Result<Vec<Placement>> {
    if document.windows.len() != images.len() {
        return Err(Error::new(E_INVALIDARG, "Session placement count mismatch"));
    }
    let mut placements = Vec::with_capacity(document.windows.len());
    for (window, images) in document.windows.iter().zip(images) {
        check(cancel)?;
        if images.unit.as_str() != window.id {
            return Err(Error::new(
                E_INVALIDARG,
                "Session placement identity mismatch",
            ));
        }
        let intrinsic = images
            .source
            .as_ref()
            .or(images.preview.as_ref())
            .map(|image| crate::crop_geometry::Size {
                width: image.width as f64,
                height: image.height as f64,
            });
        let (crop, rasterized_full) = crate::session_crop::restore_with_full_source(window)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored crop geometry"))?;
        if rasterized_full && images.source.is_none() {
            return Err(Error::new(
                E_INVALIDARG,
                "Rasterized full frame requires source PNG",
            ));
        }
        placements.push(Placement {
            node_settings: crate::unit_node_settings::Settings::read(window)
                .map_err(|message| Error::new(E_INVALIDARG, message))?,
            extension_state: crate::unit_attachment_store::Store::restore(window.extension_state),
            legacy_text: crate::sticker_extra_text::Legacy::restore(
                window.ocr_result,
                window.translation_result,
            ),
            extension_propagation: window
                .sticker_edit_propagation
                .map(|raw| std::sync::Arc::from(raw.to_owned())),
            mirror: crate::editor::MirrorState::read_raw(window.image_edit_state)
                .map_err(|message| Error::new(E_INVALIDARG, message))?,
            frame_style: crate::pin_frame_style::Style::read_raw(window.image_edit_state)
                .map_err(|message| Error::new(E_INVALIDARG, message))?,
            capture_meta: window.capture_meta.map(|value| value.get().to_owned()),
            unit: images.unit.clone(),
            art: window.is_art(),
            art_id: window.art_id.clone(),
            local_art_ports: window
                .otter_native_local_art_ports
                .map(|raw| {
                    if !window.otter_native_local_art || raw.get().len() > 16 * 1024 {
                        return Err(Error::new(E_INVALIDARG, "Invalid local Art port record"));
                    }
                    let record: crate::art_execution_grant::package::ports::Record =
                        serde_json::from_str(raw.get()).map_err(|_| {
                            Error::new(E_INVALIDARG, "Invalid local Art port record")
                        })?;
                    record
                        .validate(window.art_id.as_deref().unwrap_or_default())
                        .map_err(|_| Error::new(E_INVALIDARG, "Invalid local Art port binding"))?;
                    Ok(record)
                })
                .transpose()?,
            local_art_id: if window.otter_native_local_art {
                Some(
                    window
                        .art_id
                        .as_ref()
                        .filter(|id| {
                            window.is_art()
                                && !id.is_empty()
                                && id.len() <= 256
                                && !id.chars().any(char::is_control)
                        })
                        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid local Art identity"))?
                        .clone(),
                )
            } else {
                None
            },
            rect: window.rect(),
            minified: window.is_minified(),
            minified_views: crate::session_minified::restore(window, intrinsic)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored minified viewport"))?,
            saved_rect: window.saved_rect,
            crop_offset: window.crop_offset,
            crop,
            rasterized_full,
            opacity_normal: window.opacity(false),
            opacity_mini: window.opacity(true),
        });
    }
    Ok(placements)
}
