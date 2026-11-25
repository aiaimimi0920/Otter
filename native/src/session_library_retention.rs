//! 冻结条目追加发布前的会话原文候选；仅构造内存，成功提交由窗口 owner 决定。
use super::{candidate, Kind, Library};
use crate::{session_append, session_document, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};

/// active 是当前工作区身份集合；移除已关闭旧窗口的原文，避免它们占用窗口预算。
/// 只裁掉随旧窗口失效的原文链接；当前拓扑/几何在后续正式保存时统一采集。
pub fn prepare(
    original: &[u8],
    instance: &[u8],
    active: &[UnitId],
    kind: Kind,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    let source =
        session_document::decode(original).map_err(|error| format!("Invalid source: {error:?}"))?;
    let copy = session_document::decode(instance)
        .map_err(|error| format!("Invalid instance: {error:?}"))?;
    let library = source.library(kind)?;
    prepare_decoded(&source, &copy, active, kind, &library, entry_id, cancel)
}

/// 冻结 worker 复用实例化所借用的会话和已验证的库，不再扫描整份来源。
pub(crate) fn prepare_decoded(
    source: &session_document::Document<'_>,
    copy: &session_document::Document<'_>,
    active: &[UnitId],
    kind: Kind,
    library: &Library<'_>,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Frozen publication cancelled".into());
    }
    let entry = library
        .find(entry_id)
        .ok_or("Frozen entry no longer exists")?;
    let append = session_append::Prepared::new(source, copy, active)?;
    if append.unit_id() == entry.source_sticker_id {
        return Err("Frozen publication identity or capacity conflict".into());
    }
    if !matches_snapshot(entry.snapshot_json(), append.sticker_json())? {
        return Err("Frozen instance does not match selected snapshot".into());
    }
    let recycle = match kind {
        Kind::RecycleBin => Some(
            library
                .entries()
                .iter()
                .filter(|record| record.entry_id != entry_id)
                .map(|record| record.original)
                .collect(),
        ),
        Kind::ReferenceLibrary => None,
    };
    append.finish(recycle, cancel)
}

// 只允许实例化拥有的身份/位置/空参数差异，防止拿任意窗口消费另一条冻结记录。
fn matches_snapshot(snapshot: &str, instance: &str) -> Result<bool, String> {
    let mut expected = candidate::fields(snapshot)?;
    let mut actual = candidate::fields(instance)?;
    for key in [
        "groupId",
        "artId",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        expected.remove(key);
    }
    for key in ["id", "type", "x", "y", "params"] {
        expected.remove(key);
        actual.remove(key);
    }
    Ok(expected.len() == actual.len()
        && expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|actual| actual.get() == value.get())
        }))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_library_retention_tests.rs"]
mod tests;
