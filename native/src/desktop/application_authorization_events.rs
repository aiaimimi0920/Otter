//! Present authorization events only for the currently selected service.
use crate::{
    desktop::device_authorization_ui::View,
    device_authorization_owner::{Event, Owner},
};

pub(super) fn refresh(owner: &mut Owner, view: &mut View) {
    for event in owner.poll() {
        let (origin, status) = match event {
            Event::OfflineCleared { origin } => (
                origin,
                crate::ui_text::text("app-session-offline-cleared").into(),
            ),
            Event::Pairing { origin, .. } => {
                (origin, crate::ui_text::text("app-pairing-submitted").into())
            }
            Event::Authorized { origin, .. } => (
                origin,
                crate::ui_text::text("app-authorization-succeeded").into(),
            ),
            Event::Failed { origin, error } => (
                origin,
                crate::ui_text::format(
                    "app-authorization-failed",
                    &[("error", format!("{error:?}").into())],
                ),
            ),
        };
        if !view.closed && view.origin.as_ref() == Some(&origin) {
            view.busy = false;
            view.status = status;
        }
    }
    view.connected = view.origin.as_deref().is_some_and(|origin| {
        crate::desktop::surface_windows::now_ms()
            .is_ok_and(|now| owner.token(origin, now).is_some())
    });
}
