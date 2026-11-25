//! Display-language synchronization uses the existing fixed-origin Device-authenticated transport.
use super::*;
use crate::loom_locale_protocol::{Report, Snapshot, MAX_JSON};

impl Control<'_> {
    pub(crate) fn report_locale(&self, loom: &str, report: &Report<'_>) -> Result<Snapshot, Error> {
        let body = serde_json::to_vec(report).map_err(|_| Error::Input)?;
        if body.len() > MAX_JSON {
            return Err(Error::Input);
        }
        let bytes = self.send_limited(
            "POST",
            "/v1/otter/locale-report",
            &body,
            Duration::from_secs(5),
            MAX_JSON,
        )?;
        Snapshot::parse(
            &bytes,
            loom,
            self.session.device_id(),
            report.receiver_generation,
        )
        .map_err(|_| Error::Input)
    }
}
