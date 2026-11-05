//! 来源工作流视图：只拥有身份/索引，借用全局快照的大字段，不发送或乐观修改节点。
use super::{check, encode_payload, Error, Snapshot, WORKFLOW_ID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::AtomicBool,
};

pub struct Component<'a> {
    workflow: String,
    snapshot: &'a Snapshot,
    nodes: Vec<(usize, String)>,
    edges: Vec<usize>,
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
impl Component<'_> {
    pub fn workflow_id(&self) -> &str {
        &self.workflow
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// (当前工作区节点 ID, 来源工作流节点 ID)，只读，不更新活动文档。
    pub fn node_bindings(&self) -> impl Iterator<Item = (&str, &str)> {
        self.nodes.iter().map(|(index, target)| {
            (
                self.snapshot.value["nodes"][*index]["id"]
                    .as_str()
                    .expect("validated snapshot node"),
                target.as_str(),
            )
        })
    }
    pub fn encode_payload(&self, limit: usize, cancel: &AtomicBool) -> Result<Vec<u8>, Error> {
        self.encode(None, limit, cancel)
    }
    pub fn encode_request(
        &self,
        request_id: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Error> {
        check(cancel)?;
        super::wire::validate_request_id(request_id)?;
        self.encode(Some(request_id), limit, cancel)
    }
    fn encode(
        &self,
        request_id: Option<&str>,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Error> {
        check(cancel)?;
        #[derive(Serialize)]
        struct Node<'a> {
            id: &'a str,
            #[serde(rename = "type")]
            kind: &'a Value,
            position: &'a Value,
            data: &'a Value,
            measured: &'a Value,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Edge<'a> {
            id: &'a Value,
            source: &'a str,
            target: &'a str,
            source_handle: &'a Value,
            target_handle: &'a Value,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct View<'a> {
            nodes: Vec<Node<'a>>,
            edges: Vec<Edge<'a>>,
            viewport: &'a Value,
            document_schema_version: u32,
            document_revision: u64,
        }
        let global = &self.snapshot.value;
        let mut ids = HashMap::new();
        let mut nodes = Vec::new();
        for (index, id) in &self.nodes {
            check(cancel)?;
            let node = &global["nodes"][*index];
            ids.insert(node["id"].as_str().ok_or(Error::Node)?, id.as_str());
            nodes.push(Node {
                id,
                kind: &node["type"],
                position: &node["position"],
                // 当前 Loom 将快照整体转换/替换 YAML，未声明字段省略即保留的补丁合同。
                // 即使图片未变化，来源工作流仍须发送完整 data，不能仅按本地 ACK 省略。
                data: &node["data"],
                measured: &node["measured"],
            });
        }
        let mut edges = Vec::new();
        for index in &self.edges {
            check(cancel)?;
            let edge = &global["edges"][*index];
            edges.push(Edge {
                id: &edge["id"],
                source: ids
                    .get(edge["source"].as_str().ok_or(Error::Node)?)
                    .ok_or(Error::Node)?,
                target: ids
                    .get(edge["target"].as_str().ok_or(Error::Node)?)
                    .ok_or(Error::Node)?,
                source_handle: &edge["sourceHandle"],
                target_handle: &edge["targetHandle"],
            });
        }
        let view = View {
            nodes,
            edges,
            viewport: &global["viewport"],
            document_schema_version: 1,
            document_revision: self.snapshot.revision,
        };
        match request_id {
            Some(id) => super::wire::request(&self.workflow, &view, id, limit, cancel),
            None => encode_payload(&self.workflow, &view, limit, cancel),
        }
    }
}

/// origin 只用于来源工作流映射；节点数组必须与准备全局快照时的同一会话一致。
pub fn prepare<'a>(
    snapshot: &'a Snapshot,
    saved: &[u8],
    cancel: &AtomicBool,
) -> Result<Vec<Component<'a>>, Error> {
    check(cancel)?;
    use sha2::Digest;
    if saved.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err(Error::Session);
    }
    if <[u8; 32]>::from(sha2::Sha256::digest(saved)) != snapshot.source_digest {
        return Err(Error::Revision);
    }
    let document = crate::session_document::decode(saved).map_err(|_| Error::Session)?;
    if document.revision != snapshot.revision {
        return Err(Error::Revision);
    }
    let root: Origins = serde_json::from_slice(saved).map_err(|_| Error::Session)?;
    let records = &root.stickers;
    let nodes = snapshot.value["nodes"].as_array().ok_or(Error::Node)?;
    if records.len() != nodes.len() {
        return Err(Error::Node);
    }
    let mut index = HashMap::new();
    let mut origins = Vec::new();
    for (i, (record, node)) in records.iter().zip(nodes).enumerate() {
        let id = record.id.as_str();
        if node["id"] != id || index.insert(id, i).is_some() {
            return Err(Error::Node);
        }
        let text = |value: &Option<String>| -> Result<Option<String>, Error> {
            match value {
                None => Ok(None),
                Some(value) if value.is_empty() => Ok(None),
                Some(value) if value.len() <= 256 && !value.chars().any(char::is_control) => {
                    Ok(Some(value.clone()))
                }
                _ => Err(Error::Node),
            }
        };
        origins.push((
            text(&record.origin_workflow_id)?,
            text(&record.origin_node_id)?,
        ));
    }
    let mut adjacency = vec![Vec::new(); nodes.len()];
    for link in document.workspace.links() {
        let a = *index.get(link.from_unit.as_str()).ok_or(Error::Node)?;
        let b = *index.get(link.to_unit.as_str()).ok_or(Error::Node)?;
        adjacency[a].push(b);
        adjacency[b].push(a);
    }
    let mut visited = vec![false; nodes.len()];
    let mut components: Vec<Component<'a>> = Vec::new();
    for start in 0..nodes.len() {
        check(cancel)?;
        if visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        let mut members = Vec::new();
        let mut counts: Vec<(&str, usize)> = Vec::new();
        while let Some(i) = queue.pop_front() {
            check(cancel)?;
            members.push(i);
            if let Some(workflow) = origins[i].0.as_deref() {
                if let Some((_, count)) = counts.iter_mut().find(|(id, _)| *id == workflow) {
                    *count += 1;
                } else {
                    counts.push((workflow, 1));
                }
            }
            for next in &adjacency[i] {
                if !visited[*next] {
                    visited[*next] = true;
                    queue.push_back(*next);
                }
            }
        }
        // 严格大于才替换：票数相同保留 BFS 首次出现者，与主干选择顺序一致。
        let winner = counts
            .iter()
            .fold(None, |best: Option<(&str, usize)>, &(id, count)| {
                if best.is_none_or(|(_, n)| count > n) {
                    Some((id, count))
                } else {
                    best
                }
            });
        let Some((workflow, _)) = winner else {
            continue;
        };
        if workflow == WORKFLOW_ID {
            continue;
        } // 全局身份由全局快照唯一负责。
          // 同来源的断开分量合并发送，避免多个完整快照相互覆盖而丢失节点。
        let target = match components.iter().position(|part| part.workflow == workflow) {
            Some(i) => i,
            None => {
                components.push(Component {
                    workflow: workflow.into(),
                    snapshot,
                    nodes: Vec::new(),
                    edges: Vec::new(),
                });
                components.len() - 1
            }
        };
        let component = &mut components[target];
        for i in members {
            let id = origins[i]
                .1
                .clone()
                .unwrap_or_else(|| nodes[i]["id"].as_str().unwrap().into());
            if component.nodes.iter().any(|(_, existing)| *existing == id) {
                return Err(Error::Node);
            }
            component.nodes.push((i, id));
        }
    }
    let edges = snapshot.value["edges"].as_array().ok_or(Error::Node)?;
    for component in &mut components {
        let ids: HashSet<_> = component
            .nodes
            .iter()
            .map(|(i, _)| nodes[*i]["id"].as_str().unwrap())
            .collect();
        for (i, edge) in edges.iter().enumerate() {
            if ids.contains(edge["source"].as_str().ok_or(Error::Node)?)
                && ids.contains(edge["target"].as_str().ok_or(Error::Node)?)
            {
                component.edges.push(i);
            }
        }
    }
    check(cancel)?;
    Ok(components)
}

#[cfg(test)]
#[path = "session_workflow_component_tests.rs"]
mod tests;
