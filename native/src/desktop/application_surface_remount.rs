//! One-shot recovery entry for explicit candidates; disk hints and repeated confirmation must not become new creation requests.
use crate::{
    desktop::{
        composition::relay::sessions::Sessions, device_authorization_ui::View,
        surface_windows::Windows,
    },
    device_authorization_owner::Owner,
    live_relay_http::RecoveryCandidate,
};
use std::sync::Arc;

pub(super) fn start(
    view: &mut View,
    candidate: Arc<RecoveryCandidate>,
    authorization: &mut Owner,
    sessions: &mut Sessions,
    windows: &Windows,
    live_count: usize,
    config: impl FnOnce(
        &str,
    ) -> Result<
        crate::device_authorization_config::Config,
        crate::device_authorization_config::Error,
    >,
) -> Result<(), String> {
    if view.closed
        || view.busy
        || view.recovery_busy
        || view.mount_busy
        || view.origin.as_deref() != Some(candidate.origin())
        || !view
            .recovery_candidate
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &candidate))
    {
        return Err(crate::ui_text::text("app-recovery-candidate-changed").into());
    }
    // Consume this explicit selection; any failure requires a fresh query, retaining no candidate that a double-click could replay.
    view.recovery_candidate = None;
    if sessions.attaching() || !windows.can_mount(candidate.origin(), candidate.node()) {
        return Err(crate::ui_text::text("app-surface-already-attached").into());
    }
    if !super::live::fits_wait_budget(live_count, windows.wait_budget() + 1) {
        return Err(crate::ui_text::text("app-recovery-window-capacity").into());
    }
    let now = crate::desktop::surface_windows::now_ms().map_err(|error| error.to_string())?;
    let lease = authorization
        .lease(candidate.origin(), now)
        .map_err(|error| {
            crate::ui_text::format(
                "app-recovery-reauthorize-detail",
                &[("error", format!("{error:?}").into())],
            )
        })?;
    if lease.device_id() != candidate.device() {
        return Err(crate::ui_text::text("app-recovery-device-changed").into());
    }
    let config = config(candidate.origin()).map_err(|error| {
        crate::ui_text::format(
            "app-configuration-invalid-detail",
            &[("error", format!("{error:?}").into())],
        )
    })?;
    let request =
        crate::surface_attach_runtime::Request::recover(candidate, super::surface_start::report()?);
    sessions
        .start_attach(config, lease, Arc::new(request))
        .map_err(|error| {
            crate::ui_text::format(
                "app-recovery-start-failed",
                &[("error", error.to_string().into())],
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view(candidate: &Arc<RecoveryCandidate>) -> View {
        View {
            origin: Some(candidate.origin().into()),
            recovery_candidate: Some(candidate.clone()),
            ..Default::default()
        }
    }
    #[test]
    fn unauthed_confirmation_is_consumed_once_without_network_or_new_art() {
        let candidate = Arc::new(RecoveryCandidate::fixture());
        let mut view = view(&candidate);
        let mut sessions = Sessions::default();
        assert!(start(
            &mut view,
            candidate.clone(),
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0,
            |_| panic!("rejected recovery must not read configuration"),
        )
        .is_err());
        assert!(view.recovery_candidate.is_none());
        assert!(!sessions.attaching());
        assert!(start(
            &mut view,
            candidate,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0,
            |_| panic!("rejected recovery must not read configuration"),
        )
        .is_err());
        assert!(sessions.finished());
    }
    #[test]
    fn stale_equal_candidate_cannot_consume_new_selection_and_capacity_fails_before_network() {
        let current = Arc::new(RecoveryCandidate::fixture());
        let mut view = view(&current);
        let stale = Arc::new(RecoveryCandidate::fixture());
        let mut sessions = Sessions::default();
        assert!(start(
            &mut view,
            stale,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0,
            |_| panic!("rejected recovery must not read configuration"),
        )
        .is_err());
        assert!(Arc::ptr_eq(
            view.recovery_candidate.as_ref().unwrap(),
            &current
        ));
        let error = start(
            &mut view,
            current,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            16,
            |_| panic!("rejected recovery must not read configuration"),
        )
        .unwrap_err();
        assert_eq!(error, crate::ui_text::text("app-recovery-window-capacity"));
        assert!(view.recovery_candidate.is_none());
        assert!(sessions.finished());
    }
}
