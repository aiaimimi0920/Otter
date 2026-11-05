//! 接收路由来自对当前配对Loom的请求，不改source签名origin，不信任网络提供transport地址。
use super::{envelope, Error, Invitation};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Offline {
    pub(super) origin: String,
    pub(super) peer_id: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Transport {
    pub(super) origin: String,
}
impl Offline {
    pub(super) fn new(origin: &str, peer: &str) -> Result<Self, Error> {
        if envelope::canonical_origin(origin).as_deref() != Ok(origin)
            || !peer.strip_prefix("loom-").is_some_and(envelope::digest)
        {
            return Err(Error::Entry);
        }
        Ok(Self {
            origin: origin.into(),
            peer_id: peer.into(),
        })
    }
    pub(super) fn valid(&self) -> bool {
        Self::new(&self.origin, &self.peer_id).is_ok()
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Accept,
    Read,
    Receipt,
    Unlink,
}
pub(super) fn path(offline: bool, operation: Operation) -> &'static str {
    match (offline, operation) {
        (false, Operation::Accept) => "/v1/projections/accept",
        (false, Operation::Read) => "/v1/projections/read",
        (false, Operation::Receipt) => "/v1/projections/receipt",
        (false, Operation::Unlink) => "/v1/projections/unlink",
        (true, Operation::Accept) => "/v1/offline-projections/accept",
        (true, Operation::Read) => "/v1/offline-projections/read",
        (true, Operation::Receipt) => "/v1/offline-projections/receipt",
        (true, Operation::Unlink) => "/v1/offline-projections/unlink",
    }
}
// 缺字段兼容旧在线记录；显式null不能被Option默认为无路由而降级。
pub(super) fn some<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    de: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(de).map(Some)
}
pub(super) fn text(raw: Option<&RawValue>, expected: &str) -> bool {
    raw.is_some_and(|raw| serde_json::from_str::<&str>(raw.get()).is_ok_and(|s| s == expected))
}
impl Invitation {
    pub fn origin(&self) -> &str {
        self.offline
            .as_ref()
            .map_or(self.envelope.origin(), |route| &route.origin)
    }
    pub fn peer_id(&self) -> Option<&str> {
        self.offline.as_ref().map(|route| route.peer_id.as_str())
    }
    pub(crate) fn path(&self, operation: Operation) -> &'static str {
        path(self.offline.is_some(), operation)
    }
    pub(crate) fn validate_for(&self, origin: &str) -> Result<(), Error> {
        if self.origin() != origin || self.offline.as_ref().is_some_and(|route| !route.valid()) {
            return Err(Error::Origin);
        }
        self.envelope.validate(if self.offline.is_some() {
            self.envelope.origin()
        } else {
            origin
        })
    }
    pub(super) fn response_route(
        &self,
        route: Option<&RawValue>,
        peer: Option<&RawValue>,
        transport: Option<&RawValue>,
        delivery: Option<&RawValue>,
        device: &str,
    ) -> bool {
        if transport.is_some() {
            return false;
        }
        match &self.offline {
            None => peer.is_none() && (route.is_none() || text(route, "shared_loom")),
            Some(offline) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Target<'a> {
                    #[serde(borrow)]
                    target_device_id: &'a str,
                    #[serde(borrow)]
                    status: &'a str,
                }
                text(route, "offline_peer")
                    && text(peer, &offline.peer_id)
                    && delivery.is_some_and(|raw| {
                        serde_json::from_str::<Target<'_>>(raw.get()).is_ok_and(|target| {
                            target.target_device_id == device
                                && matches!(target.status, "accepted" | "displayed")
                        })
                    })
            }
        }
    }
}
