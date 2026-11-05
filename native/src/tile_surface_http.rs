//! Wall显示/动作固定POST；原Device nonce/HTTPS/取消与一次写语义不变。
use super::*;
use crate::tile_surface_wire::{Binding, View};
#[path = "tile_surface_action_http.rs"]
mod actions;
impl Control<'_> {
    pub(crate) fn wall_surface_open(
        &self,
        binding: &Binding,
        instance: &str,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        if !wire::identifier(instance) {
            return Err(Error::Input);
        }
        self.wall_post(
            "/v1/walls/surfaces/open",
            serde_json::json!({"binding":binding,"instanceId":instance}),
            Duration::from_secs(4),
            10 * 1024 * 1024,
        )
    }
    pub(crate) fn wall_surface_state(
        &self,
        view: &View,
        revision: u64,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.wall_post(
            "/v1/walls/surfaces/state",
            serde_json::json!({"view":view,"snapshotRevision":revision}),
            Duration::from_secs(4),
            10 * 1024 * 1024,
        )
    }
    pub(crate) fn wall_surface_close(&self, view: &View) -> Result<(), Error> {
        wall_ack(&self.wall_post(
            "/v1/walls/surfaces/close",
            serde_json::json!({"view":view}),
            Duration::from_secs(1),
            4096,
        )?)
    }
    pub(crate) fn wall_surface_image(
        &self,
        view: &View,
        id: &str,
    ) -> Result<crate::image_decode::Image, Error> {
        if !wire::image_id(id) {
            return Err(Error::Input);
        }
        let bytes = self.wall_post(
            "/v1/walls/surfaces/image",
            serde_json::json!({"view":view,"resourceId":id}),
            Duration::from_secs(8),
            24 * 1024 * 1024,
        )?;
        crate::tile_image::decode(&bytes, id, || self.cancel.is_stopped()).map_err(|_| Error::Input)
    }
}
