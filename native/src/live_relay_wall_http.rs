//! Tile 固定设备路由；所有写只发送一次，不把管理员布局 API 暴露给终端。
use super::*;
use crate::tile_wire::{
    self as wire, Endpoint, PresentationReport, PresenterLease, SceneReport, State,
};
#[path = "tile_surface_http.rs"]
mod surface;
impl Control<'_> {
    pub(crate) fn wall_state(&self) -> Result<State, Error> {
        let bytes = self.send_limited(
            "GET",
            "/v1/walls/state",
            &[],
            Duration::from_secs(3),
            10 * 1024 * 1024,
        )?;
        State::parse(&bytes).map_err(|_| Error::Input)
    }
    pub(crate) fn wall_register(&self, revision: u64, endpoint: &Endpoint) -> Result<State, Error> {
        if endpoint.device_id != self.session.device_id() {
            return Err(Error::Input);
        }
        let body =
            serde_json::to_vec(&serde_json::json!({"baseRevision":revision,"endpoint":endpoint}))
                .map_err(|_| Error::Input)?;
        let bytes = self.send_limited(
            "POST",
            "/v1/walls/endpoints/register",
            &body,
            Duration::from_secs(3),
            10 * 1024 * 1024,
        )?;
        State::parse(&bytes).map_err(|_| Error::Input)
    }
    pub(crate) fn wall_connect(&self, endpoint: &str) -> Result<PresenterLease, Error> {
        if !wire::identifier(endpoint) {
            return Err(Error::Input);
        }
        let bytes = self.wall_post(
            "/v1/walls/connect",
            serde_json::json!({"endpointId":endpoint}),
            Duration::from_secs(3),
            4096,
        )?;
        let lease: PresenterLease = serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
        if lease.protocol_version != wire::PROTOCOL
            || !wire::identifier(&lease.lease_id)
            || !(1..=60_000).contains(&lease.lease_ttl_ms)
        {
            return Err(Error::Input);
        }
        Ok(lease)
    }
    pub(crate) fn wall_heartbeat(
        &self,
        endpoint: &str,
        lease: &str,
        sequence: u64,
        applied: Option<u64>,
        presentation: Option<&PresentationReport>,
        scene: Option<&SceneReport>,
    ) -> Result<(), Error> {
        let mut body = serde_json::json!({"endpointId":endpoint,"leaseId":lease,"sequence":sequence,"appliedRevision":applied});
        if let Some(p) = presentation {
            body["presentation"] = serde_json::to_value(p).map_err(|_| Error::Input)?;
        }
        if let Some(s) = scene {
            body["scene"] = serde_json::to_value(s).map_err(|_| Error::Input)?;
        }
        let bytes = self.wall_post("/v1/walls/heartbeat", body, Duration::from_secs(3), 4096)?;
        wall_ack(&bytes)
    }
    pub(crate) fn wall_disconnect(&self, endpoint: &str, lease: &str) -> Result<(), Error> {
        let bytes = self.wall_post(
            "/v1/walls/disconnect",
            serde_json::json!({"endpointId":endpoint,"leaseId":lease}),
            Duration::from_secs(1),
            4096,
        )?;
        wall_ack(&bytes)
    }
    pub(crate) fn wall_identification_report(
        &self,
        report: &crate::tile_identification::Report,
    ) -> Result<(), Error> {
        let id = &report.identity;
        if !wire::identifier(&id.endpoint)
            || !wire::identifier(&id.lease)
            || !wire::identifier(&id.request)
        {
            return Err(Error::Input);
        }
        wall_ack(&self.wall_post(
            "/v1/walls/endpoints/identify/report",
            serde_json::json!({"endpointId":id.endpoint,"leaseId":id.lease,"requestId":id.request,"outcome":report.outcome.wire()}),
            Duration::from_secs(2), 4096,
        )?)
    }
    pub(crate) fn wall_image(
        &self,
        endpoint: &str,
        lease: &str,
        revision: u64,
        id: &str,
    ) -> Result<crate::image_decode::Image, Error> {
        if !wire::image_id(id) {
            return Err(Error::Input);
        }
        let bytes = self.wall_post("/v1/walls/images/read", serde_json::json!({"endpointId":endpoint,"leaseId":lease,"revision":revision,"resourceId":id}), Duration::from_secs(8), 24 * 1024 * 1024)?;
        crate::tile_image::decode(&bytes, id, || self.cancel.is_stopped()).map_err(|_| Error::Input)
    }
    fn wall_post(
        &self,
        path: &str,
        value: serde_json::Value,
        timeout: Duration,
        limit: usize,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let body = serde_json::to_vec(&value).map_err(|_| Error::Input)?;
        if body.len() > 512 * 1024 {
            return Err(Error::Input);
        }
        self.send_limited("POST", path, &body, timeout, limit)
    }
}
fn wall_ack(bytes: &[u8]) -> Result<(), Error> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Ack {
        protocol_version: String,
        accepted: bool,
    }
    let ack: Ack = serde_json::from_slice(bytes).map_err(|_| Error::Input)?;
    if ack.protocol_version != wire::PROTOCOL || !ack.accepted {
        return Err(Error::Input);
    }
    Ok(())
}
