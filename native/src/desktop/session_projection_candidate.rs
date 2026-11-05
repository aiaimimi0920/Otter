//! 接收图片复用会话/矢量准备；已完整验证的同一像素直接移交，避免第二次解码或另造 State。
use super::*;

pub fn prepare(
    received: crate::projection_delivery::accept::Verified,
    intent: &crate::projection_delivery::journal::Ticket,
    scope: crate::session_projection_scope::Scope,
    space: crate::session_placement::Space,
    frame: Rect,
    cancel: &AtomicBool,
) -> Result<VectorCandidate> {
    check(cancel)?;
    let unit = UnitId::parse(received.unit())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid projection receiver identity"))?;
    let (bytes, image) = received
        .into_session(intent, scope, space, frame, cancel)
        .map_err(|error| Error::new(E_INVALIDARG, format!("Projection session: {error:?}")))?;
    let images = vec![Images {
        sticker: Default::default(),
        accept_upstream: true,
        capture_original: None,
        unit,
        source: Some(image),
        preview: None,
        rasterized_annotations: None,
        restored_preview_locked: false,
    }];
    let candidate = prepare_source(bytes.into(), None, Some(images), cancel)?;
    VectorCandidate::prepare(candidate, cancel)
}
