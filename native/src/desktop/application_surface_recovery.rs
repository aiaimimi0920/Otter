//! Authentication boundary from recovery hints to read-only queries; queries neither replay creation nor automatically remount.
use super::*;
use crate::{
    desktop::composition::relay::sessions::Sessions,
    device_authorization_owner::Owner,
    surface_attach_runtime::Request,
    surface_host_capabilities::{Input, Report, Runtime},
    surface_recovery_record::Record,
};
use std::sync::Arc;

fn request(record: &Record, origin: &str, device: &str) -> std::result::Result<Request, String> {
    if record.origin() != origin || record.device() != device {
        return Err(crate::ui_text::text("app-recovery-record-mismatch").into());
    }
    let art = record
        .art()
        .ok_or(crate::ui_text::text("app-recovery-art-missing"))?;
    let report = Report::new(
        vec![Runtime::Declarative],
        vec![],
        vec![],
        vec![],
        Input::default(),
    )
    .map_err(|error| {
        crate::ui_text::format(
            "app-recovery-config-invalid",
            &[("error", format!("{error:?}").into())],
        )
    })?;
    // Art/node context for GET queries only; never pass this request to start_attach.
    Request::new(
        art.into(),
        record.node().into(),
        report,
        crate::live_relay_http::Persistence::Temporary,
    )
    .map_err(|error| error.to_string())
}
pub(super) fn query(
    record: &Record,
    selected: Option<&str>,
    auth: &mut Owner,
    sessions: &mut Sessions,
    config: impl FnOnce(
        &str,
    ) -> std::result::Result<
        crate::device_authorization_config::Config,
        crate::device_authorization_config::Error,
    >,
) -> std::result::Result<(), String> {
    if selected != Some(record.origin()) || sessions.querying_recovery() || sessions.attaching() {
        return Err(crate::ui_text::text("app-recovery-query-busy").into());
    }
    let now = crate::desktop::surface_windows::now_ms().map_err(|error| error.to_string())?;
    let lease = auth.lease(record.origin(), now).map_err(|error| {
        crate::ui_text::format(
            "app-authorization-required-detail",
            &[("error", format!("{error:?}").into())],
        )
    })?;
    let request = request(record, record.origin(), lease.device_id())?;
    let config = config(record.origin()).map_err(|error| {
        crate::ui_text::format(
            "app-configuration-invalid-detail",
            &[("error", format!("{error:?}").into())],
        )
    })?;
    sessions
        .query_surface_recovery(config, lease, Arc::new(request))
        .map_err(|error| {
            crate::ui_text::format(
                "app-recovery-query-start-failed",
                &[("error", error.to_string().into())],
            )
        })
}
impl Application {
    pub(super) fn poll_surface_recovery_ui(&mut self) {
        let mut view = self.authorization.borrow_mut();
        if let Some(result) = self.remote_sessions.take_recovery_file() {
            if !view.closed {
                match result {
                    Ok(records) => {
                        view.recovery_status = crate::ui_text::format(
                            "app-recovery-record-count",
                            &[("count", records.len().into())],
                        );
                        view.recovery = records.into_iter().map(Arc::new).collect();
                    }
                    Err(error) => {
                        view.recovery.clear();
                        view.recovery_status = crate::ui_text::format(
                            "app-recovery-file-invalid",
                            &[("error", error.to_string().into())],
                        );
                    }
                }
            }
        }
        while let Some(outcome) = self.remote_sessions.take_recovery_outcome() {
            let current = !view.closed
                && view.origin.as_deref() == Some(&outcome.origin)
                && crate::desktop::surface_windows::now_ms()
                    .ok()
                    .is_some_and(|now| {
                        self.authorization_owner
                            .lease(&outcome.origin, now)
                            .is_ok_and(|lease| lease.device_id() == outcome.device)
                    });
            if !current {
                view.recovery_candidate = None;
                if !view.closed {
                    view.recovery_status =
                        crate::ui_text::text("app-recovery-query-expired").into();
                }
                continue;
            }
            view.recovery_candidate = None;
            match outcome.result {
                Ok(Some(candidate)) => {
                    view.recovery_status =
                        crate::ui_text::text("app-recovery-candidate-found").into();
                    view.recovery_candidate = Some(Arc::new(candidate));
                }
                Ok(None) => {
                    view.recovery_status =
                        crate::ui_text::text("app-recovery-candidate-not-found").into()
                }
                Err(error) => {
                    view.recovery_status = crate::ui_text::format(
                        "app-recovery-query-incomplete",
                        &[("error", format!("{error:?}").into())],
                    )
                }
            }
        }
        view.recovery_busy = self.remote_sessions.reading_recovery_file()
            || self.remote_sessions.querying_recovery();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> Record {
        crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":"art:test","binding":null,"uncertain":true,"inFlight":false}]}"#).unwrap().remove(0)
    }
    #[test]
    fn disk_hint_only_builds_query_context_for_matching_service_and_device() {
        let record = record();
        assert!(request(&record, "https://localhost:5678", "viewer:test").is_err());
        assert!(request(&record, record.origin(), "another-device").is_err());
        let request = request(&record, record.origin(), record.device()).unwrap();
        assert_eq!(request.art(), "art:test");
        assert_eq!(request.node(), "node:test");
        assert!(record.uncertain());
    }
    #[test]
    fn untrusted_disk_record_cannot_start_network_without_current_authorization() {
        let record = record();
        let mut sessions = Sessions::default();
        assert!(query(
            &record,
            Some(record.origin()),
            &mut Owner::default(),
            &mut sessions,
            |_| panic!("unauthorized recovery must not read configuration")
        )
        .is_err());
        assert!(!sessions.querying_recovery());
        assert!(sessions.finished());
    }
}
