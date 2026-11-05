//! 离线sender沿兼容Web元数据保留独立路由；native marker固定复合目标，旧读者不能借其授在线资格。
use super::*;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Origin {
    pub(super) origin: String,
}
impl Binding {
    pub(crate) fn edit_route(&self) -> crate::projection_edit::Route {
        if self.link.offline_transport.is_some() {
            crate::projection_edit::Route::Offline
        } else {
            crate::projection_edit::Route::Shared
        }
    }
    pub(super) fn route_valid(&self) -> bool {
        match (
            &self.link.offline_transport,
            &self.target.peer_id,
            &self.link.otter_native_send.target,
        ) {
            (None, None, None) => !self.target.device_id.starts_with("peer-target:"),
            (Some(origin), Some(peer), Some(target)) => {
                origin.origin == self.envelope().origin()
                    && crate::projection_delivery::targets::Route::offline_id(
                        peer,
                        &self.target.device_id,
                    )
                    .as_ref()
                        == Some(target)
            }
            _ => false,
        }
    }
    pub(super) fn destination(&self) -> (String, String, String) {
        (
            self.envelope().origin().into(),
            self.target.peer_id.clone().unwrap_or_default(),
            self.target.device_id.clone(),
        )
    }
}
