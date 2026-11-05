//! 冻结快照的无副作用实例化；输出只供单窗口准备，绝不能作为整工作区替换命令。
use super::Kind;
use crate::{session_document::Document, workspace_id::UnitId};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Prepared {
    pub unit: UnitId,
    /// 有界、独立的单窗口会话；不是原会话的可保存替代品。
    pub bytes: crate::pixels::Pixels,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    document_schema_version: u32,
    document_revision: u64,
    stickers: [BTreeMap<String, Cow<'a, RawValue>>; 1],
    links: [(); 0],
    groups: [(); 0],
    #[serde(skip_serializing_if = "Option::is_none")]
    otter_native_coordinate_space: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Coordinates<'a> {
    #[serde(default, borrow, rename = "otterNativeCoordinateSpace")]
    value: Option<&'a RawValue>,
}

pub(super) use crate::session_json_fields::fields;

/// 调用方提供新生成的身份；发布时仍须复核当前工作区，不能把准备时检查当作锁。
/// 两种库均只读；回收站条目必须由后续成功发布事务消费，而非在这里删除。
pub fn prepare(
    source: &Document<'_>,
    kind: Kind,
    entry_id: &str,
    unit: UnitId,
    mouse: (f64, f64),
    cancel: &AtomicBool,
) -> Result<Prepared, String> {
    check(cancel)?;
    let library = source.library(kind)?;
    let entry = library.find(entry_id).ok_or("Frozen entry not found")?;
    if unit.as_str() == entry.source_sticker_id
        || source
            .windows
            .iter()
            .any(|window| window.id == unit.as_str())
    {
        return Err("Frozen copy requires a fresh unit identity".into());
    }
    let (x, y) = (mouse.0 + 50.0, mouse.1 + 50.0);
    let mut fields = fields(entry.snapshot_json())?;
    // 不能继承原窗口的分组、节点端口、执行状态或编辑传播所有权。
    for key in [
        "groupId",
        "artId",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        fields.remove(key);
    }
    for (key, value) in [
        ("id", serde_json::json!(unit.as_str())),
        ("type", serde_json::json!("sticker")),
        ("x", serde_json::json!(x)),
        ("y", serde_json::json!(y)),
        ("params", serde_json::json!({})),
    ] {
        fields.insert(
            key.into(),
            Cow::Owned(serde_json::value::to_raw_value(&value).map_err(|error| error.to_string())?),
        );
    }
    if !x.is_finite() || !y.is_finite() {
        return Err("Invalid frozen copy position".into());
    }
    let root: Coordinates =
        serde_json::from_slice(source.original_json()).map_err(|error| error.to_string())?;
    let envelope = Envelope {
        document_schema_version: 1,
        document_revision: 0,
        stickers: [fields],
        links: [],
        groups: [],
        otter_native_coordinate_space: root.value,
    };
    let bytes = crate::session_write::encode_bounded(&envelope, cancel)
        .map_err(|error| format!("Frozen candidate encoding: {error:?}"))?;
    crate::session_document::decode(bytes.as_ref())
        .map_err(|error| format!("Invalid frozen candidate: {error:?}"))?;
    check(cancel)?;
    Ok(Prepared { unit, bytes })
}

fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Frozen candidate cancelled".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_library_candidate_tests.rs"]
mod tests;
