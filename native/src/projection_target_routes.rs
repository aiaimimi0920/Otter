//! 离线目录项绑定其 peer 与远端设备；仅保存经校验的广告，不获得连接或发送权限。
use super::{envelope, name_valid, Error};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[derive(Debug)]
pub enum Route {
    SharedLoom,
    OfflinePeer {
        peer_id: String,
        peer_name: String,
        remote_device_id: String,
        advertised_available: bool,
    },
}
impl Route {
    pub(crate) fn available(&self) -> bool {
        matches!(
            self,
            Self::SharedLoom
                | Self::OfflinePeer {
                    advertised_available: true,
                    ..
                }
        )
    }
    pub(crate) fn offline_id(peer: &str, device: &str) -> Option<String> {
        (peer.strip_prefix("loom-").is_some_and(envelope::digest)
            && envelope::identifier(device)
            && !device.starts_with("peer-target:"))
        .then(|| {
            format!(
                "peer-target:{:x}",
                Sha256::digest(format!("{peer}\n{device}"))
            )
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "route", rename_all = "snake_case")]
pub(super) enum Wire {
    SharedLoom {
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    #[serde(rename_all = "camelCase")]
    OfflinePeer {
        peer_id: String,
        peer_name: String,
        remote_device_id: String,
        delivery_available: bool,
        transfer_protocol: Option<String>,
        unavailable_reason: Option<String>,
    },
}
impl Wire {
    pub(super) fn validate(self, id: &str) -> Result<Route, Error> {
        match self {
            Self::SharedLoom { extra } => {
                if id.starts_with("peer-target:") || extra.contains_key("peerId") {
                    return Err(Error::Entry);
                }
                Ok(Route::SharedLoom)
            }
            Self::OfflinePeer {
                peer_id,
                peer_name,
                remote_device_id,
                delivery_available,
                transfer_protocol,
                unavailable_reason,
            } => {
                if Route::offline_id(&peer_id, &remote_device_id).as_deref() != Some(id)
                    || !name_valid(&peer_name, 128, false)
                    || (delivery_available
                        && transfer_protocol.as_deref() != Some("loom.offline-transfer.v1"))
                    || (!delivery_available
                        && unavailable_reason.as_deref()
                            != Some("offline_peer_delivery_not_implemented"))
                {
                    return Err(Error::Entry);
                }
                Ok(Route::OfflinePeer {
                    peer_id,
                    peer_name,
                    remote_device_id,
                    advertised_available: delivery_available,
                })
            }
        }
    }
}
