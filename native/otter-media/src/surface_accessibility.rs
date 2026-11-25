//! Read-only semantic snapshot before UIA integration; holds no HWND, Host, draft, or executable action.
use crate::surface_scene::{Kind, Node, Scene};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Weak,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    pub label: Option<String>,
    pub parent: Option<usize>,
    pub disabled: bool,
}
struct Snapshot {
    identity: u64,
    alive: AtomicBool,
    entries: Vec<Entry>,
}
/// Create a distinct identity for each scene replacement so same-ID nodes or counter wraparound cannot revive old references.
pub struct Owner(Arc<Snapshot>);
#[derive(Clone)]
pub struct Handle {
    snapshot: Weak<Snapshot>,
    index: usize,
}
#[derive(Clone, Copy)]
pub enum Direction {
    Parent,
    FirstChild,
    LastChild,
    NextSibling,
    PreviousSibling,
}
impl Owner {
    pub fn new(scene: &Scene) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        // Never wrap and reuse UIA runtime identities; reject construction at u64 exhaustion rather than revive old nodes.
        let identity = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("Surface accessibility identity exhausted");
        let mut entries = Vec::new();
        collect(scene.root(), None, &mut entries);
        Self(Arc::new(Snapshot {
            identity,
            alive: AtomicBool::new(true),
            entries,
        }))
    }
    pub fn node(&self, id: &str) -> Option<Handle> {
        if !self.0.alive.load(Ordering::Acquire) {
            return None;
        }
        let index = self.0.entries.iter().position(|entry| entry.id == id)?;
        Some(Handle {
            snapshot: Arc::downgrade(&self.0),
            index,
        })
    }
    pub fn root(&self) -> Option<Handle> {
        self.node(&self.0.entries.first()?.id)
    }
    pub fn revoke(&self) {
        self.0.alive.store(false, Ordering::Release);
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.revoke();
    }
}
impl Handle {
    pub fn runtime_id(&self) -> Option<[i32; 3]> {
        let snapshot = self.snapshot.upgrade()?;
        if !snapshot.alive.load(Ordering::Acquire) {
            return None;
        }
        Some([
            snapshot.identity as i32,
            (snapshot.identity >> 32) as i32,
            self.index as i32,
        ])
    }
    pub fn relative(&self, direction: Direction) -> Option<Self> {
        let snapshot = self.snapshot.upgrade()?;
        if !snapshot.alive.load(Ordering::Acquire) {
            return None;
        }
        let parent = snapshot.entries.get(self.index)?.parent;
        let index = match direction {
            Direction::Parent => parent?,
            Direction::FirstChild => snapshot
                .entries
                .iter()
                .position(|entry| entry.parent == Some(self.index))?,
            Direction::LastChild => snapshot
                .entries
                .iter()
                .rposition(|entry| entry.parent == Some(self.index))?,
            Direction::NextSibling => (self.index + 1..snapshot.entries.len())
                .find(|&index| snapshot.entries[index].parent == parent)?,
            Direction::PreviousSibling => (0..self.index)
                .rev()
                .find(|&index| snapshot.entries[index].parent == parent)?,
        };
        Some(Self {
            snapshot: self.snapshot.clone(),
            index,
        })
    }
    /// Return only a static description valid at call time, not action authorization or proof of the current edit value.
    /// Concurrent revocation allows reads already started to finish; all reads started after revocation fail.
    pub fn read(&self) -> Option<Entry> {
        let snapshot = self.snapshot.upgrade()?;
        if !snapshot.alive.load(Ordering::Acquire) {
            return None;
        }
        Some(snapshot.entries.get(self.index)?.clone())
    }
}
fn collect(node: &Node, parent: Option<usize>, entries: &mut Vec<Entry>) {
    if !node.visible() {
        return;
    }
    let index = entries.len();
    entries.push(Entry {
        disabled: node.kind() != Kind::Image
            && node.props().get("disabled") == Some(&serde_json::Value::Bool(true)),
        id: node.id().to_owned(),
        kind: node.kind(),
        // The legacy image branch passes only alt, without expanding general accessibleProps or accepting disabled.
        label: if node.kind() == Kind::Image {
            node.props()
                .get("alt")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        } else {
            node.accessibility()
                .get("label")
                .and_then(|v| v.as_str())
                .or_else(|| match node.kind() {
                    Kind::Text => node.props().get("text").and_then(|v| v.as_str()),
                    Kind::Button => node.props().get("label").and_then(|v| v.as_str()),
                    _ => None,
                })
                .map(str::to_owned)
        },
        parent,
    });
    // Scene already limits inputs to 2048 nodes, 32 levels, and 1 MiB; do not copy props, events, or input values.
    for child in node.visible_children() {
        collect(child, Some(index), entries);
    }
}

#[cfg(test)]
#[path = "surface_accessibility_tests.rs"]
mod tests;
