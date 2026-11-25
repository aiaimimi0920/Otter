//! Offline senders preserve independent routes in Web-compatible metadata; native markers fix composite targets, and old readers cannot grant online eligibility from them.
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
        if self.is_invitation() {
            return self.target.is_none()
                && self.link.offline_transport.is_none()
                && self.link.otter_native_send.target.is_none()
                && self.link.linked == self.link.receiver_device_id.is_some()
                && self.link.linked == self.link.receiver_unit_id.is_some()
                && self
                    .link
                    .receiver_device_id
                    .as_deref()
                    .is_none_or(envelope::identifier)
                && self
                    .link
                    .receiver_unit_id
                    .as_deref()
                    .is_none_or(envelope::identifier);
        }
        let Some(target) = &self.target else {
            return false;
        };
        match (
            &self.link.offline_transport,
            &target.peer_id,
            &self.link.otter_native_send.target,
        ) {
            (None, None, None) => !target.device_id.starts_with("peer-target:"),
            (Some(origin), Some(peer), Some(marker_target)) => {
                origin.origin == self.envelope().origin()
                    && crate::projection_delivery::targets::Route::offline_id(
                        peer,
                        &target.device_id,
                    )
                    .as_ref()
                        == Some(marker_target)
            }
            _ => false,
        }
    }
    pub(super) fn destination(&self) -> (String, String, Option<String>) {
        (
            self.envelope().origin().into(),
            self.target
                .as_ref()
                .and_then(|target| target.peer_id.clone())
                .unwrap_or_default(),
            self.target.as_ref().map(|target| target.device_id.clone()),
        )
    }
}
