//! 六种既有v1操作的不可变请求；prepare后字节固定，重试不得换op/session/base版本。
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub object_id: String,
    pub value: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Operation {
    Read {
        projection_id: String,
    },
    Initialize {
        projection_id: String,
        session_id: String,
        expected_digest: String,
        objects: Objects,
    },
    Attach {
        projection_id: String,
        session_id: String,
    },
    Mode {
        projection_id: String,
        session_id: String,
        op_id: String,
        base_mode_revision: u64,
        mode: Mode,
    },
    Apply {
        projection_id: String,
        session_id: String,
        op_id: String,
        base_revision: u64,
        mode_revision: u64,
        changes: Vec<Change>,
    },
    Checkpoint {
        projection_id: String,
        session_id: String,
        expected_revision: u64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    operation: Operation,
    bytes: Vec<u8>,
}
impl Request {
    pub fn prepare(operation: Operation) -> Result<Self, Error> {
        let bytes = serde_json::to_vec(&operation).map_err(|_| Error::Json)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let request = Self { operation, bytes };
        if !projection(request.projection_id()) || request.session_id().is_some_and(|s| !session(s))
        {
            return Err(Error::Identity);
        }
        match &request.operation {
            Operation::Initialize {
                expected_digest,
                objects,
                ..
            } => {
                if !hex(expected_digest, 64) {
                    return Err(Error::Basis);
                }
                objects::validate(objects)?;
            }
            Operation::Mode {
                op_id,
                base_mode_revision,
                ..
            } if !identifier(op_id) || !revision(*base_mode_revision) => {
                return Err(Error::Revision)
            }
            Operation::Apply {
                op_id,
                base_revision,
                mode_revision,
                changes,
                ..
            } => {
                if !identifier(op_id)
                    || !revision(*base_revision)
                    || !revision(*mode_revision)
                    || mode_revision > base_revision
                {
                    return Err(Error::Revision);
                }
                if changes.is_empty() || changes.len() > 32 {
                    return Err(Error::Limit);
                }
                let mut ids = std::collections::HashSet::new();
                for c in changes {
                    if !identifier(&c.object_id) || !ids.insert(&c.object_id) {
                        return Err(Error::Identity);
                    }
                    if !c.value.is_null() && !objects::annotation(&c.object_id, &c.value) {
                        return Err(Error::Annotation);
                    }
                }
            }
            Operation::Checkpoint {
                expected_revision, ..
            } if !revision(*expected_revision) => return Err(Error::Revision),
            _ => {}
        }
        Ok(request)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        Self::prepare(json::decode(bytes, MAX_BYTES)?)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    pub fn is_read(&self) -> bool {
        matches!(self.operation, Operation::Read { .. })
    }
    pub fn projection_id(&self) -> &str {
        match &self.operation {
            Operation::Read { projection_id }
            | Operation::Initialize { projection_id, .. }
            | Operation::Attach { projection_id, .. }
            | Operation::Mode { projection_id, .. }
            | Operation::Apply { projection_id, .. }
            | Operation::Checkpoint { projection_id, .. } => projection_id,
        }
    }
    pub fn session_id(&self) -> Option<&str> {
        match &self.operation {
            Operation::Read { .. } => None,
            Operation::Initialize { session_id, .. }
            | Operation::Attach { session_id, .. }
            | Operation::Mode { session_id, .. }
            | Operation::Apply { session_id, .. }
            | Operation::Checkpoint { session_id, .. } => Some(session_id),
        }
    }
    pub fn response(&self, bytes: &[u8], previous: Option<&Document>) -> Result<Document, Error> {
        let doc = Document::parse(bytes)?;
        if self.session_id().is_some_and(|id| id != doc.session_id()) {
            return Err(Error::Identity);
        }
        if let Operation::Initialize {
            expected_digest, ..
        } = &self.operation
        {
            if expected_digest != doc.basis().digest() {
                return Err(Error::Basis);
            }
        }
        if let Some(previous) = previous {
            doc.follows(previous)?;
        }
        // 幂等重试可能返回后来已被其他操作修改的当前文档，不能要求value仍等于原payload。
        // 但每种成功操作必须跨过其原围栏；HTTP 200不能把未执行的旧文档确认掉。
        let accepted = match &self.operation {
            Operation::Mode {
                base_mode_revision, ..
            } => doc.mode_revision() > *base_mode_revision,
            Operation::Apply {
                base_revision,
                mode_revision,
                ..
            } => doc.revision() > *base_revision && doc.mode_revision() == *mode_revision,
            Operation::Checkpoint {
                expected_revision, ..
            } => expected_revision.checked_add(1) == Some(doc.checkpoint_revision()),
            _ => true,
        };
        if !accepted {
            return Err(Error::Revision);
        }
        Ok(doc)
    }
}
impl Serialize for Request {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.operation.serialize(s)
    }
}
impl<'de> Deserialize<'de> for Request {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::prepare(Operation::deserialize(d)?)
            .map_err(|_| serde::de::Error::custom("invalid edit request"))
    }
}
