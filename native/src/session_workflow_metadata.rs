//! 工作流来源采用计划与归档提示；只生成身份数据，不改文档、不写资源索引或文件。
use super::{check, components, Error, Snapshot, WORKFLOW_ID};
use serde::{Deserialize, Serialize};
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
    /// 应用采用前须匹配原文；提示本身不代表图片资源已归档或保存成功。
    pub fn matches(&self, saved: &[u8]) -> bool {
        use sha2::Digest;
        saved.len() <= crate::session_document::MAX_SESSION_BYTES
            && <[u8; 32]>::from(sha2::Sha256::digest(saved)) == self.source_digest
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Origin {
    id: String,
    origin_workflow_id: Option<String>,
    origin_node_id: Option<String>,
}
#[derive(Deserialize)]
struct Origins {
    stickers: Vec<Origin>,
}

pub fn prepare(snapshot: &Snapshot, saved: &[u8], cancel: &AtomicBool) -> Result<Plan, Error> {
    // 包括摘要、版本、全局身份、来源 ID 冲突验证；任何失败不返回部分提示。
    let components = components::prepare(snapshot, saved, cancel)?;
    let origins: Origins = serde_json::from_slice(saved).map_err(|_| Error::Session)?;
    let origins: BTreeMap<_, _> = origins
        .stickers
        .iter()
        .map(|origin| (origin.id.as_str(), origin))
        .collect();
    let mut hints = ArchiveHints {
        workflows: BTreeMap::new(),
    };
    let mut global = WorkflowHint {
        nodes: BTreeMap::new(),
    };
    for id in origins.keys() {
        check(cancel)?;
        global.nodes.insert(
            (*id).into(),
            NodeHint {
                sticker_id: (*id).into(),
            },
        );
    }
    hints.workflows.insert(WORKFLOW_ID.into(), global);
    let mut adoptions = Vec::new();
    for component in components {
        check(cancel)?;
        let mut hint = WorkflowHint {
            nodes: BTreeMap::new(),
        };
        for (local, target) in component.node_bindings() {
            check(cancel)?;
            hint.nodes.insert(
                target.into(),
                NodeHint {
                    sticker_id: local.into(),
                },
            );
            let current = origins.get(local).ok_or(Error::Node)?;
            // 主干只在工作流归属变化时采用；同工作流缺省 nodeId 仍以本地 ID 解析。
            if current.origin_workflow_id.as_deref() != Some(component.workflow_id()) {
                adoptions.push(Adoption {
                    sticker_id: local.into(),
                    origin_workflow_id: component.workflow_id().into(),
                    origin_node_id: current
                        .origin_node_id
                        .as_deref()
                        .filter(|id| !id.is_empty())
                        .unwrap_or(target)
                        .into(),
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
    })
}

#[cfg(test)]
#[path = "session_workflow_metadata_tests.rs"]
mod tests;
