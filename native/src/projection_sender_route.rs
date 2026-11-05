//! 显式发送路由只来自固定目录/持久原意图；不接受HTTP回复提供地址或更换对等身份。
use super::*;
use crate::projection_delivery::targets::{Route, Target};
use serde_json::value::RawValue;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Offline {
    pub(super) peer_id: String,
    pub(super) remote_device_id: String,
}
impl Selection {
    pub(crate) fn for_target(
        scope: &str,
        origin: &str,
        unit: &str,
        target: &Target,
    ) -> Result<Self, Error> {
        match target.route() {
            Route::SharedLoom => Self::new(scope, origin, unit, target.id()),
            Route::OfflinePeer {
                peer_id,
                remote_device_id,
                advertised_available: true,
                ..
            } => Self::offline(scope, origin, unit, target.id(), peer_id, remote_device_id),
            _ => Err(Error::Format),
        }
    }
    pub(super) fn offline(
        scope: &str,
        origin: &str,
        unit: &str,
        target: &str,
        peer: &str,
        remote: &str,
    ) -> Result<Self, Error> {
        if Route::offline_id(peer, remote).as_deref() != Some(target) {
            return Err(Error::Format);
        }
        // 借用字段先核小尺寸；与在线选择共用scope/origin/unit准入，再安装显式路由。
        let mut value = Self::new(scope, origin, unit, remote)?;
        value.target = target.into();
        value.offline = Some(Offline {
            peer_id: peer.into(),
            remote_device_id: remote.into(),
        });
        value.validate()?;
        Ok(value)
    }
    pub(super) fn destination(&self) -> (String, String, String) {
        (
            self.origin.clone(),
            self.offline.as_ref().map_or("", |r| &r.peer_id).into(),
            self.offline
                .as_ref()
                .map_or(self.target.as_str(), |r| &r.remote_device_id)
                .into(),
        )
    }
}
impl Intent {
    pub(crate) fn path(&self, operation: Operation) -> &'static str {
        match (self.record.selection.offline.is_some(), operation) {
            (false, Operation::Read) => "/v1/projections/read",
            (false, Operation::Create) => "/v1/projections/create",
            (false, Operation::Update) => "/v1/projections/update",
            (false, Operation::Unlink) => "/v1/projections/unlink",
            (true, Operation::Read) => "/v1/offline-projections/read",
            (true, Operation::Create) => "/v1/offline-projections/create",
            (true, Operation::Update) => "/v1/offline-projections/update",
            (true, Operation::Unlink) => "/v1/offline-projections/unlink",
        }
    }
    pub(super) fn receiver(&self) -> &str {
        self.record
            .selection
            .offline
            .as_ref()
            .map_or(self.target(), |r| &r.remote_device_id)
    }
    pub(super) fn response_route(&self, route: Option<&RawValue>, peer: Option<&RawValue>) -> bool {
        let text = |raw: Option<&RawValue>, expected: &str| {
            raw.is_some_and(|raw| {
                serde_json::from_str::<&str>(raw.get()).is_ok_and(|s| s == expected)
            })
        };
        match &self.record.selection.offline {
            None => peer.is_none() && (route.is_none() || text(route, "shared_loom")),
            Some(offline) => text(route, "offline_peer") && text(peer, &offline.peer_id),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Read,
    Create,
    Update,
    Unlink,
}

// 与缺字段区分：显式null不能把离线路由静默降为SharedLoom。
pub(super) fn present<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<&'de RawValue>, D::Error> {
    <&RawValue>::deserialize(de).map(Some)
}
pub(super) fn decode(
    raw: Option<&RawValue>,
    scope: &str,
    origin: &str,
    unit: &str,
    target: &str,
) -> Result<Selection, Error> {
    match raw {
        None => Selection::new(scope, origin, unit, target),
        Some(raw) => {
            if raw.get().len() > 1024 {
                return Err(Error::Limit);
            }
            let route: Offline = serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
            Selection::offline(
                scope,
                origin,
                unit,
                target,
                &route.peer_id,
                &route.remote_device_id,
            )
        }
    }
}
#[cfg(test)]
#[path = "projection_sender_route_tests.rs"]
mod tests;
