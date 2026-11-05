//! 权威文档必须完整校验后交付；固定basis、session及单调版本不由HTTP成功替代。
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    digest: String,
    width: u32,
    height: u32,
}
impl Basis {
    pub fn new(digest: &str, width: u32, height: u32) -> Result<Self, Error> {
        if !hex(digest, 64) {
            return Err(Error::Basis);
        }
        let value = Self {
            digest: digest.into(),
            width,
            height,
        };
        value.valid().then_some(value).ok_or(Error::Basis)
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub(super) fn valid(&self) -> bool {
        hex(&self.digest, 64)
            && (1..=8192).contains(&self.width)
            && (1..=8192).contains(&self.height)
            && u64::from(self.width) * u64::from(self.height) <= 16_777_216
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Object {
    revision: u64,
    value: serde_json::Value,
}
impl Object {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn value(&self) -> &serde_json::Value {
        &self.value
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    schema: String,
    session_id: String,
    basis: Basis,
    revision: u64,
    mode_revision: u64,
    checkpoint_revision: u64,
    receipt_count: u16,
    mode: Mode,
    objects: BTreeMap<String, Object>,
}
impl Document {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let doc: Self = json::decode(bytes, MAX_BYTES)?;
        doc.validate()?;
        Ok(doc)
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        if self.schema != "neuro.projection-edit.v1" || !session(&self.session_id) {
            return Err(Error::Identity);
        }
        if !self.basis.valid() {
            return Err(Error::Basis);
        }
        if !revision(self.revision)
            || !revision(self.mode_revision)
            || !revision(self.checkpoint_revision)
            || self.mode_revision > self.revision
            || self.checkpoint_revision > self.mode_revision
        {
            return Err(Error::Revision);
        }
        if self.receipt_count > 256
            || self.objects.len() > 256
            || serde_json::to_vec(self).map_or(true, |v| v.len() > MAX_BYTES)
        {
            return Err(Error::Limit);
        }
        for (id, item) in &self.objects {
            if !identifier(id) || !revision(item.revision) || item.revision > self.revision {
                return Err(Error::Revision);
            }
            if !item.value.is_null() && !objects::annotation(id, &item.value) {
                return Err(Error::Annotation);
            }
        }
        Ok(())
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn basis(&self) -> &Basis {
        &self.basis
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn mode_revision(&self) -> u64 {
        self.mode_revision
    }
    pub fn checkpoint_revision(&self) -> u64 {
        self.checkpoint_revision
    }
    pub fn receipt_count(&self) -> u16 {
        self.receipt_count
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn objects(&self) -> &BTreeMap<String, Object> {
        &self.objects
    }
    pub fn live_objects(&self) -> Objects {
        self.objects
            .iter()
            .filter(|(_, v)| !v.value.is_null())
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
    pub fn follows(&self, previous: &Self) -> Result<(), Error> {
        if self.session_id != previous.session_id {
            return Err(Error::Identity);
        }
        if self.basis != previous.basis {
            return Err(Error::Basis);
        }
        if self.revision < previous.revision
            || self.mode_revision < previous.mode_revision
            || self.checkpoint_revision < previous.checkpoint_revision
            || (self.revision == previous.revision && self != previous)
        {
            return Err(Error::Revision);
        }
        Ok(())
    }
}
