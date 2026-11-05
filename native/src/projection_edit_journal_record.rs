//! 本地编辑事务持久形状；不保存设备token/私钥，逐项及总编码预算均核真实字节。
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Pending {
    pub route: Route,
    pub request: Request,
    pub previous: Option<Document>,
    pub view: Objects,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected: Option<super::super::Code>,
}
impl Pending {
    pub(super) fn validate(&self) -> Result<(), Error> {
        if self.rejected.is_some_and(|code| !code.recoverable())
            || (self.rejected.is_some()
                && (!matches!(
                    self.request.operation(),
                    super::super::Operation::Apply { .. }
                        | super::super::Operation::Mode { .. }
                        | super::super::Operation::Checkpoint { .. }
                ) || self.previous.is_none()))
        {
            return Err(Error::Format);
        }
        if self.request.is_read() {
            return Err(Error::Format);
        }
        super::super::objects::validate(&self.view).map_err(|_| Error::Format)?;
        if let Some(doc) = &self.previous {
            doc.validate().map_err(|_| Error::Format)?;
            if self.request.session_id() != Some(doc.session_id()) {
                return Err(Error::Changed);
            }
            let matches = match self.request.operation() {
                super::super::Operation::Initialize {
                    expected_digest, ..
                } => expected_digest == doc.basis().digest(),
                super::super::Operation::Mode {
                    base_mode_revision, ..
                } => *base_mode_revision == doc.mode_revision(),
                super::super::Operation::Apply {
                    base_revision,
                    mode_revision,
                    ..
                } => *base_revision == doc.revision() && *mode_revision == doc.mode_revision(),
                super::super::Operation::Checkpoint {
                    expected_revision, ..
                } => *expected_revision == doc.revision(),
                _ => true,
            };
            if !matches {
                return Err(Error::Changed);
            }
        } else if !matches!(
            self.request.operation(),
            super::super::Operation::Initialize { .. } | super::super::Operation::Attach { .. }
        ) {
            return Err(Error::Changed);
        }
        if let super::super::Operation::Initialize { objects, .. } = self.request.operation() {
            if objects != &self.view {
                return Err(Error::Changed);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Entry {
    pub context: Context,
    pub generation: u64,
    pub document: Option<Document>,
    pub pending: Option<Pending>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<super::super::source::Frozen>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Ledger {
    schema_version: u32,
    pub entries: Vec<Entry>,
}
impl Ledger {
    pub(super) fn new(entries: Vec<Entry>) -> Self {
        Self {
            schema_version: 1,
            entries,
        }
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let doc: Self = super::super::json::decode(bytes, MAX_BYTES).map_err(|_| Error::Format)?;
        doc.validate()?;
        Ok(doc)
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1 {
            return Err(Error::Format);
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err(Error::Limit);
        }
        for (index, entry) in self.entries.iter().enumerate() {
            entry.context.validate()?;
            if !super::super::revision(entry.generation)
                || (entry.document.is_none() && entry.pending.is_none() && entry.source.is_none())
                || self.entries[..index]
                    .iter()
                    .any(|e| e.context.same_key(&entry.context))
            {
                return Err(Error::Format);
            }
            if serde_json::to_vec(entry).map_or(true, |v| v.len() > MAX_ENTRY) {
                return Err(Error::Limit);
            }
            if let Some(doc) = &entry.document {
                doc.validate().map_err(|_| Error::Format)?;
            }
            if let Some(source) = &entry.source {
                source.validate().map_err(|_| Error::Format)?;
                if entry.document.as_ref().is_some_and(|doc| {
                    doc.session_id() != source.session() || doc.basis() != &source.basis()
                }) || entry
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.request.session_id() != Some(source.session()))
                {
                    return Err(Error::Changed);
                }
                if let Some(p) = &entry.pending {
                    if let super::super::Operation::Initialize {
                        projection_id,
                        expected_digest,
                        objects,
                        ..
                    } = p.request.operation()
                    {
                        if projection_id != source.anchor()
                            || expected_digest != source.basis().digest()
                            || objects != source.initial()
                        {
                            return Err(Error::Changed);
                        }
                    }
                }
            }
            if let Some(p) = &entry.pending {
                p.validate()?;
                if entry.document != p.previous {
                    return Err(Error::Changed);
                }
            }
        }
        Ok(())
    }
}
