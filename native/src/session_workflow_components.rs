//! Source-workflow views own only identities/indexes and borrow large fields from global snapshots, without sending or optimistic node mutation.
use super::{check, encode_payload, Error, Snapshot, WORKFLOW_ID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::AtomicBool,
};

pub struct Component<'a> {
    layout: Layout,
    snapshot: &'a Snapshot,
}
/// Retain only bounded identities/indexes, not source content, images, or snapshot references.
#[derive(Clone)]
pub(super) struct Layout {
    workflow: String,
    nodes: Vec<(usize, String)>,
    edges: Vec<usize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Origin {
    pub(super) id: String,
    pub(super) origin_workflow_id: Option<String>,
    pub(super) origin_node_id: Option<String>,
}
#[derive(Deserialize)]
struct Origins {
    stickers: Vec<Origin>,
}
impl Component<'_> {
    pub fn workflow_id(&self) -> &str {
        self.layout.workflow_id()
    }
    pub fn node_count(&self) -> usize {
        self.layout.nodes.len()
    }
    /// (current workspace node ID, source workflow node ID), read-only and without active-document updates.
    pub fn node_bindings(&self) -> impl Iterator<Item = (&str, &str)> {
        self.layout.indexed_bindings().map(|(index, target)| {
            (
                self.snapshot.value["nodes"][index]["id"]
                    .as_str()
                    .expect("validated snapshot node"),
                target,
            )
        })
    }
    pub fn encode_payload(&self, limit: usize, cancel: &AtomicBool) -> Result<Vec<u8>, Error> {
        self.layout.encode(self.snapshot, None, limit, cancel)
    }
    pub fn encode_request(
        &self,
        request_id: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Error> {
        self.layout
            .encode_request(self.snapshot, request_id, limit, cancel)
    }
}
impl Layout {
    pub(super) fn workflow_id(&self) -> &str {
        &self.workflow
    }
    pub(super) fn indexed_bindings(&self) -> impl Iterator<Item = (usize, &str)> {
        self.nodes
            .iter()
            .map(|(index, target)| (*index, target.as_str()))
    }
    pub(super) fn encode_request(
        &self,
        snapshot: &Snapshot,
        request_id: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Error> {
        check(cancel)?;
        super::wire::validate_request_id(request_id)?;
        self.encode(snapshot, Some(request_id), limit, cancel)
    }
    fn encode(
        &self,
        snapshot: &Snapshot,
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
        let global = &snapshot.value;
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
                // Current Loom converts/replaces YAML from the complete snapshot; it declares no patch contract preserving omitted fields.
                // Even unchanged images require full data for source workflows; local ACK alone cannot justify omission.
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
            document_revision: snapshot.revision,
        };
        match request_id {
            Some(id) => super::wire::request(&self.workflow, &view, id, limit, cancel),
            None => encode_payload(&self.workflow, &view, limit, cancel),
        }
    }
}

/// origin maps source workflows only; the node array must match the same session used to prepare the global snapshot.
pub fn prepare<'a>(
    snapshot: &'a Snapshot,
    saved: &[u8],
    cancel: &AtomicBool,
) -> Result<Vec<Component<'a>>, Error> {
    let (layouts, _) = prepare_with_origins(snapshot, saved, cancel)?;
    Ok(layouts
        .into_iter()
        .map(|layout| Component { layout, snapshot })
        .collect())
}

type Prepared = (Vec<Layout>, Vec<Origin>);

pub(super) fn prepare_with_origins(
    snapshot: &Snapshot,
    saved: &[u8],
    cancel: &AtomicBool,
) -> Result<Prepared, Error> {
    validate_source(snapshot, saved, cancel)?;
    let document = crate::session_document::decode(saved).map_err(|_| Error::Session)?;
    from_verified_source(snapshot, &document, cancel)
}

pub(super) fn prepare_from(
    snapshot: &Snapshot,
    document: &crate::session_document::Document<'_>,
    cancel: &AtomicBool,
) -> Result<Prepared, Error> {
    validate_source(snapshot, document.original_json(), cancel)?;
    from_verified_source(snapshot, document, cancel)
}

fn validate_source(snapshot: &Snapshot, saved: &[u8], cancel: &AtomicBool) -> Result<(), Error> {
    check(cancel)?;
    use sha2::Digest;
    if saved.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err(Error::Session);
    }
    if <[u8; 32]>::from(sha2::Sha256::digest(saved)) != snapshot.source_digest {
        return Err(Error::Revision);
    }
    Ok(())
}

fn from_verified_source(
    snapshot: &Snapshot,
    document: &crate::session_document::Document<'_>,
    cancel: &AtomicBool,
) -> Result<Prepared, Error> {
    if document.revision != snapshot.revision {
        return Err(Error::Revision);
    }
    // Keep independent field deserialization so duplicate or nonstring source fields still fail.
    let Origins {
        stickers: mut origins,
    } = serde_json::from_slice(document.original_json()).map_err(|_| Error::Session)?;
    let nodes = snapshot.value["nodes"].as_array().ok_or(Error::Node)?;
    if origins.len() != nodes.len() {
        return Err(Error::Node);
    }
    let mut index = HashMap::new();
    for (i, (record, node)) in origins.iter_mut().zip(nodes).enumerate() {
        check(cancel)?;
        let id = node["id"].as_str().ok_or(Error::Node)?;
        if record.id != id || index.insert(id, i).is_some() {
            return Err(Error::Node);
        }
        for field in [&mut record.origin_workflow_id, &mut record.origin_node_id] {
            match field.as_deref() {
                None => (),
                Some("") => *field = None,
                Some(value) if value.len() <= 256 && !value.chars().any(char::is_control) => (),
                _ => return Err(Error::Node),
            }
        }
    }
    let mut adjacency = vec![Vec::new(); nodes.len()];
    for link in document.workspace.links() {
        let a = *index.get(link.from_unit.as_str()).ok_or(Error::Node)?;
        let b = *index.get(link.to_unit.as_str()).ok_or(Error::Node)?;
        adjacency[a].push(b);
        adjacency[b].push(a);
    }
    let mut visited = vec![false; nodes.len()];
    let mut membership = vec![None; nodes.len()];
    let mut components: Vec<Layout> = Vec::new();
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
            if let Some(workflow) = origins[i].origin_workflow_id.as_deref() {
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
        // Replace only on strictly greater votes: ties retain the first BFS occurrence, matching mainline selection order.
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
        } // The global snapshot exclusively owns global identity.
          // Merge disconnected same-source components into one send, preventing complete snapshots from overwriting each other and losing nodes.
        let target = match components.iter().position(|part| part.workflow == workflow) {
            Some(i) => i,
            None => {
                components.push(Layout {
                    workflow: workflow.into(),
                    nodes: Vec::new(),
                    edges: Vec::new(),
                });
                components.len() - 1
            }
        };
        let component = &mut components[target];
        for i in members {
            let origin = &origins[i];
            let id = origin.origin_node_id.as_deref().unwrap_or(&origin.id);
            component.nodes.push((i, id.into()));
            membership[i] = Some(target);
        }
    }
    let edges = snapshot.value["edges"].as_array().ok_or(Error::Node)?;
    for component in &components {
        let mut ids = HashSet::with_capacity(component.nodes.len());
        if component.nodes.iter().any(|(_, id)| !ids.insert(id)) {
            return Err(Error::Node);
        }
    }
    // Distribute edges once in global order; both endpoints must belong to the same source view.
    if !components.is_empty() {
        let part = |id: &str| index.get(id).and_then(|&i| membership[i]);
        for (i, edge) in edges.iter().enumerate() {
            check(cancel)?;
            if let Some(owner) = part(edge["source"].as_str().ok_or(Error::Node)?) {
                if part(edge["target"].as_str().ok_or(Error::Node)?) == Some(owner) {
                    components[owner].edges.push(i);
                }
            }
        }
    }
    check(cancel)?;
    Ok((components, origins))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_component_tests.rs"]
mod tests;
