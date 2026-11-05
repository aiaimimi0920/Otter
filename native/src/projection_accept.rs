//! accept 的相关响应与内联 PNG；借用 wire 解码，原压缩字节及像素均受共享预算约束。
use super::{envelope, receive_route, Envelope, Error, Invitation};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const MAX_RESPONSE: usize = 6 * 1024 * 1024;
pub const MAX_PNG: usize = 4 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 16_777_216;

pub struct Received {
    envelope: Envelope,
    revision: u64,
    digest: String,
    unit: String,
    device: String,
    width: u32,
    height: u32,
    png: crate::pixels::Pixels,
    offline: Option<receive_route::Offline>,
    editing: Option<crate::projection_edit::Document>,
}
pub struct Verified {
    pub(super) received: Received,
    pub(super) image: crate::image_decode::Image,
    pub(super) intent_id: Option<String>,
}
#[path = "projection_receive_edit.rs"]
mod edit;
#[path = "projection_receive_session.rs"]
mod session;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire<'a> {
    envelope: Envelope,
    revision: u64,
    digest: String,
    width: u32,
    height: u32,
    linked: bool,
    receiver_device_id: String,
    receiver_unit_id: String,
    #[serde(borrow)]
    snapshot: Option<Snapshot<'a>>,
    #[serde(default, borrow)]
    editing: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    route: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    peer_id: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    offline_transport: Option<&'a serde_json::value::RawValue>,
    #[serde(borrow)]
    delivery: Option<&'a serde_json::value::RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot<'a> {
    width: u32,
    height: u32,
    #[serde(borrow)]
    image_base64: &'a str,
}
impl Invitation {
    pub fn receiver_unit_id(&self) -> String {
        format!("delivery-{}", &self.envelope.projection_id()[11..])
    }
    pub(crate) fn accept_body(&self, origin: &str) -> Result<Vec<u8>, Error> {
        self.validate_for(origin)?;
        // accepted/过期邀请仍可恢复丢失的成功响应；服务器拥有最终签名/期限/规则裁决。
        serde_json::to_vec(&serde_json::json!({"envelope":self.envelope,
            "expectedRevision":self.revision,"expectedDigest":self.digest,
            "receiverUnitId":self.receiver_unit_id(),"confirmed":true}))
        .map_err(|_| Error::Json)
    }
}
impl Received {
    pub(crate) fn parse(
        bytes: &[u8],
        invitation: &Invitation,
        origin: &str,
        device: &str,
    ) -> Result<Self, Error> {
        if bytes.len() > MAX_RESPONSE {
            return Err(Error::Limit);
        }
        invitation.validate_for(origin)?;
        let wire: Wire<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        wire.envelope.validate(invitation.envelope.origin())?;
        if wire.envelope != invitation.envelope
            || !invitation.response_route(
                wire.route,
                wire.peer_id,
                wire.offline_transport,
                wire.delivery,
                device,
            )
            || !wire.linked
            || wire.receiver_device_id != device
            || !envelope::identifier(device)
            || wire.receiver_unit_id != invitation.receiver_unit_id()
            || !(invitation.revision..=envelope::MAX_REVISION).contains(&wire.revision)
            || !envelope::digest(&wire.digest)
        {
            return Err(Error::Response);
        }
        Self::from_wire(wire, invitation.offline.clone())
    }
    fn from_wire(wire: Wire<'_>, offline: Option<receive_route::Offline>) -> Result<Self, Error> {
        let snapshot = wire.snapshot.ok_or(Error::Snapshot)?;
        let editing = edit::parse(wire.editing, &wire.digest, wire.width, wire.height)?;
        if snapshot.width == 0
            || snapshot.height == 0
            || snapshot.width > 8192
            || snapshot.height > 8192
            || u64::from(snapshot.width) * u64::from(snapshot.height) > MAX_PIXELS
            || (wire.width, wire.height) != (snapshot.width, snapshot.height)
            || snapshot.image_base64.is_empty()
            || snapshot.image_base64.len() > MAX_PNG.div_ceil(3) * 4
        {
            return Err(Error::Snapshot);
        }
        // 预算覆盖解码目的容量，不保留 wire/base64 的第二份字符串。
        let capacity = snapshot.image_base64.len().div_ceil(4) * 3;
        let mut lease = crate::pixels::reserve(capacity).ok_or(Error::Budget)?;
        let mut png = Vec::new();
        png.try_reserve_exact(capacity).map_err(|_| Error::Budget)?;
        if !lease.grow_to(png.capacity()) {
            return Err(Error::Budget);
        }
        STANDARD
            .decode_vec(snapshot.image_base64, &mut png)
            .map_err(|_| Error::Snapshot)?;
        if png.len() > MAX_PNG
            || !png.starts_with(b"\x89PNG\r\n\x1a\n")
            || format!("{:x}", Sha256::digest(&png)) != wire.digest
        {
            return Err(Error::Snapshot);
        }
        Ok(Self {
            envelope: wire.envelope,
            revision: wire.revision,
            digest: wire.digest,
            unit: wire.receiver_unit_id,
            device: wire.receiver_device_id,
            width: snapshot.width,
            height: snapshot.height,
            png: crate::pixels::Pixels::from_reserved(png, lease),
            offline,
            editing,
        })
    }
    /// 当前线程必须已初始化 COM；只有完整解码/尺寸一致后的值才可进入会话准备。
    pub fn verify(self, stopped: impl Fn() -> bool) -> Result<Verified, Error> {
        if stopped() {
            return Err(Error::Stopped);
        }
        let image = crate::image_decode::Image::from_projection_png(
            &self.png,
            self.width,
            self.height,
            &stopped,
        )
        .map_err(|_| {
            if stopped() {
                Error::Stopped
            } else {
                Error::Snapshot
            }
        })?;
        if stopped() {
            return Err(Error::Stopped);
        }
        Ok(Verified {
            received: self,
            image,
            intent_id: None,
        })
    }
}
impl Verified {
    /// 仅接收 worker 在原请求完成后绑定一次；旧结果不能被重贴到同投送的新意图。
    pub(crate) fn bind_intent(mut self, ticket: &super::journal::Ticket) -> Result<Self, Error> {
        if self.device() != ticket.device()
            || self.unit() != ticket.unit()
            || self.envelope() != ticket.envelope()
            || self.received.offline != ticket.invitation().offline
            || self.revision() < ticket.invitation().revision()
            || self
                .intent_id
                .as_deref()
                .is_some_and(|id| id != ticket.intent_id())
        {
            return Err(Error::Response);
        }
        self.intent_id = Some(ticket.intent_id().into());
        Ok(self)
    }
    pub(crate) fn device(&self) -> &str {
        &self.received.device
    }
    pub fn envelope(&self) -> &Envelope {
        &self.received.envelope
    }
    pub fn revision(&self) -> u64 {
        self.received.revision
    }
    pub fn digest(&self) -> &str {
        &self.received.digest
    }
    pub fn unit(&self) -> &str {
        &self.received.unit
    }
    pub fn image(&self) -> &crate::image_decode::Image {
        &self.image
    }
    pub fn png(&self) -> &[u8] {
        &self.received.png
    }
    pub fn editing(&self) -> Option<&crate::projection_edit::Document> {
        self.received.editing.as_ref()
    }
}
impl std::fmt::Debug for Received {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReceivedProjection")
            .field("unit", &self.unit)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Verified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.received.fmt(f)
    }
}
#[cfg(test)]
#[path = "projection_receive_edit_tests.rs"]
pub(crate) mod edit_tests;
#[cfg(test)]
#[path = "projection_accept_tests.rs"]
pub(crate) mod tests;

#[path = "projection_receive_read.rs"]
mod read;
pub(crate) use read::Read;
