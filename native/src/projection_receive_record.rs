//! 接收写前意图的有界格式；没有 token/图片，不把本地记录当新的远端接受授权。
use super::*;
use crate::projection_delivery::{envelope, receive_route, Envelope, Invitation};
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: usize = 128;
pub const MAX_BYTES: usize = 512 * 1024;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Record {
    pub(super) intent_id: String,
    pub(super) envelope: Envelope,
    expected_revision: u64,
    expected_digest: String,
    pub(super) device: String,
    pub(super) unit: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(super) cleanup: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "receive_route::some"
    )]
    pub(super) offline: Option<receive_route::Offline>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    version: u32,
    pub(super) entries: Vec<Record>,
}
impl Record {
    pub(super) fn new(invitation: &Invitation, device: &str) -> Result<Self, Error> {
        let guid =
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(|_| Error::Write)?;
        let value = Self {
            intent_id: format!("{guid:?}"),
            envelope: invitation.envelope().clone(),
            expected_revision: invitation.revision(),
            expected_digest: invitation.digest().into(),
            device: device.into(),
            unit: invitation.receiver_unit_id(),
            cleanup: false,
            offline: invitation.offline.clone(),
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), Error> {
        self.envelope
            .validate(self.envelope.origin())
            .map_err(|_| Error::Format)?;
        if !intent_id(&self.intent_id)
            || self.offline.as_ref().is_some_and(|route| !route.valid())
            || !envelope::identifier(&self.device)
            || self.unit != format!("delivery-{}", &self.envelope.projection_id()[11..])
            || !(self.envelope.source_revision()..=envelope::MAX_REVISION)
                .contains(&self.expected_revision)
            || !envelope::digest(&self.expected_digest)
        {
            return Err(Error::Format);
        }
        Ok(())
    }
    pub(super) fn same_key(&self, other: &Self) -> bool {
        self.envelope.origin() == other.envelope.origin()
            && self.envelope.projection_id() == other.envelope.projection_id()
    }
    pub(super) fn same_intent(&self, other: &Self) -> bool {
        self.intent_id == other.intent_id
            && self.envelope == other.envelope
            && self.expected_revision == other.expected_revision
            && self.expected_digest == other.expected_digest
            && self.device == other.device
            && self.unit == other.unit
            && self.offline == other.offline
    }
    pub(super) fn invitation(&self) -> Invitation {
        Invitation {
            envelope: self.envelope.clone(),
            revision: self.expected_revision,
            digest: self.expected_digest.clone(),
            source_name: self.envelope.source_device().into(),
            accepted: true,
            offline: self.offline.clone(),
        }
    }
}
pub(crate) fn intent_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, c)| {
            if [8, 13, 18, 23].contains(&index) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
impl Document {
    pub(super) fn new(entries: Vec<Record>) -> Self {
        Self {
            version: 1,
            entries,
        }
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| Error::Format)?;
        if value.version != 1 || value.entries.len() > MAX_ENTRIES {
            return Err(Error::Format);
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        for entry in &value.entries {
            entry.validate()?;
            if !ids.insert(&entry.intent_id)
                || !keys.insert((entry.envelope.origin(), entry.envelope.projection_id()))
            {
                return Err(Error::Format);
            }
        }
        Ok(value)
    }
}
