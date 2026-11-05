//! 原生协作编辑合同与写前事务；不拥有Loom合并权威，也不自动启用产品双向入口。
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[path = "projection_edit_document.rs"]
mod document;
#[path = "projection_edit_json.rs"]
pub(crate) mod json;
#[path = "projection_edit_objects.rs"]
mod objects;
#[path = "projection_edit_request.rs"]
mod request;
pub use document::{Basis, Document, Object};
pub use request::{Change, Operation, Request};
#[cfg(windows)]
#[path = "projection_edit_checkpoint.rs"]
pub(crate) mod checkpoint;
#[path = "projection_edit_editor.rs"]
pub mod editor;
#[cfg(windows)]
#[path = "projection_edit_journal.rs"]
pub mod journal;
#[cfg(windows)]
#[path = "projection_edit_source.rs"]
pub(crate) mod source;
#[path = "projection_edit_view.rs"]
pub mod view;
#[cfg(windows)]
#[path = "projection_edit_workspace.rs"]
pub mod workspace;
pub const MAX_BYTES: usize = 256 * 1024;
pub const MAX_REVISION: u64 = 9_007_199_254_740_991;
pub type Objects = BTreeMap<String, serde_json::Value>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Limit,
    Identity,
    Annotation,
    Revision,
    Basis,
    Response,
    Stopped,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    OneWay,
    TwoWay,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Shared,
    Offline,
}
impl Route {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::Shared => "/v1/projections/edit",
            Self::Offline => "/v1/offline-projections/edit",
        }
    }
}
pub(crate) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && !matches!(value, "__proto__" | "constructor" | "prototype")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(crate) fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn session(value: &str) -> bool {
    value.strip_prefix("edit:").is_some_and(|v| hex(v, 32))
}
pub(crate) fn projection(value: &str) -> bool {
    value
        .strip_prefix("projection:")
        .is_some_and(|v| hex(v, 32))
}
pub(crate) fn revision(value: u64) -> bool {
    (1..=MAX_REVISION).contains(&value)
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Transport(crate::live_relay_http::Error),
    Protocol(Error),
    Journal(journal::Error),
    Rejected { status: u16, code: Code },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    NotFound,
    ObjectConflict,
    ModeConflict,
    RevisionConflict,
    SessionMismatch,
    BasisConflict,
    ReadOnly,
    LogFull,
    ObjectBudget,
    OperationReused,
    SnapshotLocked,
    Busy,
    Other,
}
impl Code {
    /// 只包含服务端明确未接受的编辑冲突；超时、未知状态、busy及身份错误不提供退休资格。
    pub(crate) fn recoverable(self) -> bool {
        matches!(
            self,
            Self::ObjectConflict
                | Self::ModeConflict
                | Self::RevisionConflict
                | Self::ReadOnly
                | Self::LogFull
                | Self::ObjectBudget
        )
    }
    pub(crate) fn parse(status: u16, value: &str) -> Self {
        match (status, value) {
            (404, "projection_edit_not_found") => Self::NotFound,
            (409, "projection_edit_object_conflict") => Self::ObjectConflict,
            (409, "projection_edit_mode_conflict") => Self::ModeConflict,
            (409, "projection_edit_revision_conflict") => Self::RevisionConflict,
            (409, "projection_edit_session_mismatch") => Self::SessionMismatch,
            (409, "projection_edit_basis_conflict") => Self::BasisConflict,
            (403, "projection_edit_read_only") => Self::ReadOnly,
            (413, "projection_edit_log_full") => Self::LogFull,
            (413, "projection_edit_object_budget") => Self::ObjectBudget,
            (409, "projection_edit_operation_reused") => Self::OperationReused,
            (409, "projection_edit_snapshot_locked") => Self::SnapshotLocked,
            (429, "projection_busy") => Self::Busy,
            _ => Self::Other,
        }
    }
}
#[cfg(test)]
#[path = "projection_edit_test_fixture.rs"]
pub(crate) mod fixture;
#[cfg(all(test, windows))]
#[path = "projection_edit_joint_tests.rs"]
mod joint;
#[cfg(test)]
#[path = "projection_edit_tests.rs"]
mod tests;
