//! 后台冻结实例化输入；不可变来源随任务同行，不携带 HWND 或隐式资源路径授权。
use crate::{session_library::Kind, session_source::Source, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Input {
    pub source: Source,
    pub kind: Kind,
    pub entry_id: String,
    pub unit: UnitId,
    /// 已在请求时从物理桌面位置转换为会话逻辑坐标。
    pub mouse: (f64, f64),
}
impl Input {
    pub(super) fn valid(&self) -> bool {
        self.source.len() <= crate::session_document::MAX_SESSION_BYTES
            && UnitId::parse(&self.entry_id).is_some()
            && self.mouse.0.is_finite()
            && self.mouse.1.is_finite()
    }
    pub(super) fn prepare(self, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
        if cancel.load(Ordering::Acquire) {
            return Err("Frozen preparation cancelled".into());
        }
        let document = crate::session_document::decode(&self.source)
            .map_err(|error| format!("Frozen source rejected: {error:?}"))?;
        let candidate = crate::session_library::candidate::prepare(
            &document,
            self.kind,
            &self.entry_id,
            self.unit,
            self.mouse,
            cancel,
        )?;
        // Candidate 的现有 owner 接收 Vec；仅复制所选单窗口，而非整个库/会话。
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(candidate.bytes.len())
            .map_err(|_| "Cannot allocate frozen worker input")?;
        bytes.extend_from_slice(candidate.bytes.as_ref());
        if cancel.load(Ordering::Acquire) {
            return Err("Frozen preparation cancelled".into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "session_frozen_input_tests.rs"]
mod tests;
