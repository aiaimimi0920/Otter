//! 后台移交已验证的接收像素和协作矢量；不在 UI 线程扫描内联图片或再建 Art 状态。
use super::*;

pub(in crate::desktop) fn prepare(
    received: crate::projection_delivery::accept::Verified,
    intent: &crate::projection_delivery::journal::Ticket,
    scope: crate::session_projection_scope::Scope,
    space: crate::session_placement::Space,
    frame: Rect,
    target: crate::session_append::Target<'_>,
    cancel: &AtomicBool,
) -> Result<(
    PreparedRestore,
    crate::pixels::Pixels,
    crate::projection_delivery::binding::Binding,
)> {
    check(cancel)?;
    let unit = UnitId::parse(received.unit())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid projection receiver identity"))?;
    let (bytes, image, binding) = received
        .into_session(intent, scope, space, frame, cancel)
        .map_err(|error| Error::new(E_INVALIDARG, format!("Projection session: {error:?}")))?;
    let (restore, retained) =
        PreparedRestore::projected_sticker(bytes, unit, image, cancel, |document| {
            target
                .append(document, cancel)
                .map_err(|error| Error::new(E_INVALIDARG, error))
        })?;
    Ok((restore, retained, binding))
}
