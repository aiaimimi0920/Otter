//! Share value tokens with buttons without replacing still-in-flight modal publications; failures must not fall back and replay motion.
use super::*;

pub fn motion_native(
    target: &Destination,
    packet: Message,
    timeout: u32,
    modal: &mut Modal,
) -> Result<()> {
    if !matches!(packet.id, WM_MOUSEMOVE | WM_MOUSEHOVER) {
        return Err(Error::InvalidPayload);
    }
    modal.check_result()?;
    modal.require_idle()?;
    target.preflight().map_err(Error::Destination)?;
    if target.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    let owner = modal.owner.ok_or(Error::InvalidPayload)?;
    let token = NEXT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::ModalPending)?;
    let publication =
        native_request::Publication::publish(owner, target, packet, token, modal.popup_scope)?;
    send_to_with_timeout(target, publication.message()?, timeout)?;
    if !publication.finished() {
        return Err(Error::DeliveryFailed);
    }
    Ok(())
}
#[cfg(test)]
#[path = "live_input_native_motion_tests.rs"]
mod tests;
