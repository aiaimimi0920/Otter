//! 后台精确停止 patch；完整 typed binding + canonical allocation 核验由调用者保持，不吞未知字段。
use super::*;
use serde_json::Value;
pub(crate) struct Acknowledgement {
    selection: Selection,
    envelope: Envelope,
    _source: Source,
}
impl Acknowledgement {
    /// 仅 canonical owner 在真实 commit 后调用；候选/手动 Save 不得取得此 proof。
    pub(crate) fn committed(
        index: &checkpoint::Index,
        source: &Source,
        scope: &str,
        intent: &Intent,
    ) -> Option<Self> {
        if !index.is_for(source)
            || scope != intent.record.selection.scope
            || index.for_intent(intent).is_some()
        {
            return None;
        }
        Some(Self {
            selection: intent.record.selection.clone(),
            envelope: intent.envelope().clone(),
            _source: source.clone(),
        })
    }
    pub(crate) fn matches(&self, intent: &Intent) -> bool {
        self.selection == intent.record.selection && self.envelope == *intent.envelope()
    }
}
/// 返回的 Source 只是候选；调用者必须等默认工作区 checkpoint 的真实包含/移除证明。
pub(crate) fn prepare(
    source: &Source,
    expected: &Binding,
    remove: bool,
    cancel: &AtomicBool,
) -> Result<Source, Error> {
    check(cancel)?;
    let index = checkpoint::Index::capture(source).ok_or(Error::Format)?;
    if !index.includes(expected) || (remove && !expected.link.stop_pending) || expected.link.stopped
    {
        return Err(Error::Changed);
    }
    let result = patch::transform(source, expected.unit(), cancel, |object| {
        let raw = object.get("projectionSenders").ok_or(Error::Changed)?;
        let entries: inventory::Entries<'_> =
            serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
        let mut result = Vec::with_capacity(entries.0.len());
        let mut found = false;
        for raw in entries.0 {
            check(cancel)?;
            let mut value: Value = serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
            if inventory::native(raw.get())?
                && Binding::decode(raw.get(), expected.unit(), expected.scope())? == *expected
            {
                if found {
                    return Err(Error::Changed);
                }
                found = true;
                if remove {
                    continue;
                }
                value["link"]["stopPending"] = true.into();
            }
            result.push(value);
        }
        if !found {
            return Err(Error::Changed);
        }
        Ok(result)
    })?;
    checkpoint::Index::capture(&result).ok_or(Error::Format)?;
    check(cancel)?;
    Ok(result)
}
