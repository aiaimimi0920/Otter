//! 发送响应核原签名身份/目标/单调版本；协作文档只绑定同digest/尺寸，普通发送流程仍明确拒绝。
use super::*;
use serde::Deserialize;
use serde_json::value::RawValue;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire<'a> {
    #[serde(borrow)]
    envelope: &'a RawValue,
    revision: u64,
    #[serde(borrow)]
    digest: &'a str,
    linked: bool,
    #[serde(borrow)]
    receiver_device_id: Option<&'a str>,
    #[serde(borrow)]
    receiver_unit_id: Option<&'a str>,
    #[serde(borrow)]
    delivery: Delivery<'a>,
    #[serde(borrow)]
    snapshot: Option<snapshot::Wire<'a>>,
    #[serde(borrow)]
    editing: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "route::present")]
    route: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "route::present")]
    peer_id: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "route::present")]
    offline_transport: Option<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Delivery<'a> {
    #[serde(borrow)]
    target_device_id: &'a str,
    status: Status,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    AwaitingConfirmation,
    Accepted,
    Displayed,
    Rejected,
}
pub struct Response {
    revision: u64,
    digest: String,
    receiver_device: Option<String>,
    receiver_unit: Option<String>,
    status: Status,
    editing: Option<crate::projection_edit::Document>,
}
impl Response {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn receiver_device(&self) -> Option<&str> {
        self.receiver_device.as_deref()
    }
    pub fn receiver_unit(&self) -> Option<&str> {
        self.receiver_unit.as_deref()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub(crate) fn editing(&self) -> Option<&crate::projection_edit::Document> {
        self.editing.as_ref()
    }
    pub(crate) fn parse(
        bytes: &[u8],
        intent: &Intent,
        created: bool,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        check(cancel)?;
        if bytes.len() > super::super::accept::MAX_RESPONSE {
            return Err(Error::Limit);
        }
        let wire: Wire<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
        if wire.envelope.get().len() > 8192 {
            return Err(Error::Limit);
        }
        let envelope: Envelope =
            serde_json::from_str(wire.envelope.get()).map_err(|_| Error::Response)?;
        if &envelope != intent.envelope()
            || !intent.response_route(wire.route, wire.peer_id)
            || wire.offline_transport.is_some()
            || !wire.linked
            || wire.delivery.target_device_id != intent.target()
            || wire.delivery.status == Status::Rejected
            || !(envelope.source_revision()..=super::super::envelope::MAX_REVISION)
                .contains(&wire.revision)
            || !super::super::envelope::digest(wire.digest)
            || (wire.revision == envelope.source_revision() && wire.digest != intent.digest())
            || (created
                && (wire.revision != envelope.source_revision() || wire.digest != intent.digest()))
            || wire.receiver_device_id.is_some() != wire.receiver_unit_id.is_some()
            || (wire.delivery.status == Status::AwaitingConfirmation)
                != wire.receiver_device_id.is_none()
            || wire.receiver_device_id.is_some_and(|id| {
                !super::super::envelope::identifier(id) || id != intent.receiver()
            })
            || wire
                .receiver_unit_id
                .is_some_and(|id| !super::super::envelope::identifier(id))
        {
            return Err(Error::Response);
        }
        let editing = wire
            .editing
            .map(|raw| {
                crate::projection_edit::Document::parse(raw.get().as_bytes())
                    .map_err(|_| Error::Response)
            })
            .transpose()?;
        if editing
            .as_ref()
            .is_some_and(|doc| doc.basis().digest() != wire.digest)
        {
            return Err(Error::Response);
        }
        if let Some(snapshot) = wire.snapshot {
            let checked = snapshot::Snapshot::decode(snapshot, cancel)?;
            if checked.digest != wire.digest
                || editing
                    .as_ref()
                    .is_some_and(|doc| doc.basis().size() != (checked.width, checked.height))
            {
                return Err(Error::Snapshot);
            }
        }
        check(cancel)?;
        Ok(Self {
            revision: wire.revision,
            digest: wire.digest.into(),
            receiver_device: wire.receiver_device_id.map(str::to_owned),
            receiver_unit: wire.receiver_unit_id.map(str::to_owned),
            status: wire.delivery.status,
            editing,
        })
    }
}
impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionSenderResponse")
            .field("revision", &self.revision)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
#[cfg(test)]
#[path = "projection_sender_response_tests.rs"]
mod tests;
