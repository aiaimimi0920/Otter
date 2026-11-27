//! Shortcut exchange is outbound only and restricted to the authenticated Device origin.
use super::*;
use crate::loom_shortcut_protocol::{Report, Snapshot, MAX_JSON};
impl Control<'_> {
    pub(crate) fn report_shortcuts(&self, loom: &str, report: &Report) -> Result<Snapshot, Error> {
        let body = serde_json::to_vec(report).map_err(|_| Error::Input)?;
        if body.len() > MAX_JSON {
            return Err(Error::Input);
        }
        let bytes = self.send_limited(
            "POST",
            "/v1/otter/shortcut-report",
            &body,
            Duration::from_secs(5),
            MAX_JSON,
        )?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Clock)?
            .as_millis()
            .try_into()
            .map_err(|_| Error::Clock)?;
        Snapshot::parse(
            &bytes,
            loom,
            self.session.device_id(),
            report.receiver_generation.as_deref(),
            now,
        )
        .map_err(|_| Error::Input)
    }
}
