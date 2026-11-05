//! Wall view 的身份/版本合同；不伪造普通 Surface Binding，输入许可由独立动作owner复核。
use crate::tile_wire::{self as wall, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
#[path = "tile_surface_snapshot.rs"]
mod snapshot;
pub(crate) use snapshot::Snapshot;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Binding {
    pub endpoint_id: String,
    pub lease_id: String,
    pub revision: u64,
}
impl From<&crate::tile_shared::Key> for Binding {
    fn from(k: &crate::tile_shared::Key) -> Self {
        Self {
            endpoint_id: k.endpoint.clone(),
            lease_id: k.lease.clone(),
            revision: k.revision,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct View {
    pub binding: Binding,
    pub instance_id: String,
    pub attachment_id: String,
}
pub(crate) struct State {
    pub view: View,
    pub size: [u32; 2],
    pub generation: u64,
    pub sequence: u64,
    pub preview_revision: u64,
    pub result_revision: u64,
    pub snapshot: Arc<Snapshot>,
    pub activity: Arc<crate::tile_surface_activity::Activity>,
}
fn require(ok: bool) -> Result<()> {
    ok.then_some(()).ok_or("tile_surface_invalid")
}
pub(super) fn integer(v: &Value) -> Result<u64> {
    v.as_u64()
        .filter(|n| *n <= wall::MAX_SAFE)
        .ok_or("tile_surface_integer")
}
pub(super) fn id(v: &Value) -> Result<&str> {
    v.as_str()
        .filter(|s| wall::identifier(s))
        .ok_or("tile_surface_identity")
}
// JSON 已受10MiB HTTP门；进一步限制嵌套、累计键/字符串与节点，防大属性绕过场景门。
fn bounded(root: &Value) -> Result<()> {
    let mut stack = vec![(root, 0)];
    let (mut nodes, mut chars) = (0, 0);
    while let Some((v, depth)) = stack.pop() {
        nodes += 1;
        require(nodes <= 32_768 && depth <= 32)?;
        match v {
            Value::String(s) => chars += s.encode_utf16().count(),
            Value::Object(o) => {
                require(o.len() + stack.len() <= 32_768)?;
                for (k, v) in o {
                    chars += k.encode_utf16().count();
                    stack.push((v, depth + 1));
                }
            }
            Value::Array(a) => {
                require(a.len() + stack.len() <= 32_768)?;
                stack.extend(a.iter().map(|v| (v, depth + 1)));
            }
            _ => {}
        }
        require(chars <= 2 * 1024 * 1024)?;
    }
    Ok(())
}
fn commit_revision(v: &Value, key: &str, instance: &str, generation: u64) -> Result<u64> {
    if v.is_null() {
        return Ok(0);
    }
    require(v["protocolVersion"] == "loom.surface.v1" && v["instanceId"] == instance)?;
    id(&v["requestId"])?;
    require(integer(&v["generation"])? <= generation)?;
    integer(&v[key])
}
impl State {
    pub fn parse(
        bytes: &[u8],
        binding: &Binding,
        instance: &str,
        previous: Option<&Self>,
    ) -> Result<Self> {
        require(bytes.len() <= 10 * 1024 * 1024)?;
        let v: Value = serde_json::from_slice(bytes).map_err(|_| "tile_surface_json")?;
        bounded(&v)?;
        let view: View =
            serde_json::from_value(v["view"].clone()).map_err(|_| "tile_surface_view")?;
        require(
            v["protocolVersion"] == wall::PROTOCOL
                && view.binding == *binding
                && view.instance_id == instance
                && wall::identifier(&view.attachment_id),
        )?;
        let width = integer(&v["width"])?;
        let height = integer(&v["height"])?;
        require((1..=4096).contains(&width) && (1..=4096).contains(&height))?;
        let generation = integer(&v["generation"])?;
        // Loom新建实例从0开始；这里只核安全整数，换代仍由下方单调/完整view围栏约束。
        let sequence = integer(&v["sequence"])?;
        let preview_revision =
            commit_revision(&v["preview"], "previewRevision", instance, generation)?;
        let result_revision =
            commit_revision(&v["result"], "resultRevision", instance, generation)?;
        let activity = Arc::new(crate::tile_surface_activity::Activity::parse(&v, &view)?);
        let snapshot = if v["snapshot"].is_null() {
            previous
                .ok_or("tile_surface_missing_snapshot")?
                .snapshot
                .clone()
        } else {
            let next = snapshot::parse(&v["snapshot"], &view)?;
            if let Some(old) = previous.filter(|old| old.snapshot.revision == next.revision) {
                require(old.snapshot.digest == next.digest)?;
                old.snapshot.clone()
            } else {
                Arc::new(next)
            }
        };
        if let Some(old) = previous {
            require(
                view == old.view
                    && [width as u32, height as u32] == old.size
                    && generation >= old.generation
                    && sequence >= old.sequence
                    && snapshot.revision >= old.snapshot.revision
                    && preview_revision >= old.preview_revision
                    && result_revision >= old.result_revision,
            )?;
        }
        Ok(Self {
            view,
            size: [width as u32, height as u32],
            generation,
            sequence,
            preview_revision,
            result_revision,
            snapshot,
            activity,
        })
    }
}
#[cfg(test)]
#[path = "tile_surface_wire_tests.rs"]
mod tests;
