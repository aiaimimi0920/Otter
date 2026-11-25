//! 工作流来源采用计划与归档提示；只生成身份数据，不改文档、不写资源索引或文件。
use super::{check, components, Error, Snapshot, WORKFLOW_ID};
use serde::Serialize;
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
#[path = "session_workflow_archive.rs"]
pub mod archive;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Adoption {
    pub sticker_id: String,
    pub origin_workflow_id: String,
    pub origin_node_id: String,
}
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeHint {
    pub sticker_id: String,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct WorkflowHint {
    pub nodes: BTreeMap<String, NodeHint>,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct ArchiveHints {
    pub workflows: BTreeMap<String, WorkflowHint>,
}
pub struct Plan {
    source_digest: [u8; 32],
    revision: u64,
    adoptions: Vec<Adoption>,
    hints: ArchiveHints,
    layouts: Vec<components::Layout>,
}
impl Plan {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn adoptions(&self) -> &[Adoption] {
        &self.adoptions
    }
    pub fn archive_hints(&self) -> &ArchiveHints {
        &self.hints
    }
    /// 发送只借用这份准备结果；快照仍须是同一份保存原文及修订。
    pub(super) fn layouts_for(
        &self,
        snapshot: &Snapshot,
        cancel: &AtomicBool,
    ) -> Result<&[components::Layout], Error> {
        check(cancel)?;
        if snapshot.revision != self.revision || snapshot.source_digest != self.source_digest {
            return Err(Error::Revision);
        }
        Ok(&self.layouts)
    }
    /// 应用采用前须匹配原文；提示本身不代表图片资源已归档或保存成功。
    pub fn matches(&self, saved: &[u8]) -> bool {
        use sha2::Digest;
        saved.len() <= crate::session_document::MAX_SESSION_BYTES
            && <[u8; 32]>::from(sha2::Sha256::digest(saved)) == self.source_digest
    }
}
pub fn prepare(snapshot: &Snapshot, saved: &[u8], cancel: &AtomicBool) -> Result<Plan, Error> {
    let (components, origins) = components::prepare_with_origins(snapshot, saved, cancel)?;
    assemble(snapshot, components, origins, cancel)
}

pub(crate) fn prepare_from(
    snapshot: &Snapshot,
    document: &crate::session_document::Document<'_>,
    cancel: &AtomicBool,
) -> Result<Plan, Error> {
    let (components, origins) = components::prepare_from(snapshot, document, cancel)?;
    assemble(snapshot, components, origins, cancel)
}

fn assemble(
    snapshot: &Snapshot,
    layouts: Vec<components::Layout>,
    origins: Vec<components::Origin>,
    cancel: &AtomicBool,
) -> Result<Plan, Error> {
    // 只使用已核对 ID 与窗口顺序的来源字段，不重新遍历保存原文。
    let mut hints = ArchiveHints {
        workflows: BTreeMap::new(),
    };
    let mut global = WorkflowHint {
        nodes: BTreeMap::new(),
    };
    for origin in &origins {
        check(cancel)?;
        global.nodes.insert(
            origin.id.clone(),
            NodeHint {
                sticker_id: origin.id.clone(),
            },
        );
    }
    hints.workflows.insert(WORKFLOW_ID.into(), global);
    let mut adoptions = Vec::new();
    for component in &layouts {
        check(cancel)?;
        let mut hint = WorkflowHint {
            nodes: BTreeMap::new(),
        };
        for (index, target) in component.indexed_bindings() {
            check(cancel)?;
            let current = &origins[index];
            let local = &current.id;
            hint.nodes.insert(
                target.into(),
                NodeHint {
                    sticker_id: local.into(),
                },
            );
            // 主干只在工作流归属变化时采用；同工作流缺省 nodeId 仍以本地 ID 解析。
            if current.origin_workflow_id.as_deref() != Some(component.workflow_id()) {
                adoptions.push(Adoption {
                    sticker_id: local.into(),
                    origin_workflow_id: component.workflow_id().into(),
                    origin_node_id: target.into(),
                });
            }
        }
        hints.workflows.insert(component.workflow_id().into(), hint);
    }
    check(cancel)?;
    Ok(Plan {
        source_digest: snapshot.source_digest,
        revision: snapshot.revision,
        adoptions,
        hints,
        layouts,
    })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_metadata_tests.rs"]
mod tests;
