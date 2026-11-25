//! 后台冻结实例化输入；不可变来源随任务同行，不携带 HWND 或隐式资源路径授权。
use crate::desktop::session_candidate::PreparedRestore;
use crate::{
    session_library::{self, Kind},
    session_source::Source,
    workspace_id::UnitId,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Input {
    pub source: Source,
    pub kind: Kind,
    pub entry_id: String,
    pub unit: UnitId,
    /// 已在请求时从物理桌面位置转换为会话逻辑坐标。
    pub mouse: (f64, f64),
    /// 与应用恢复凭证在同一次 UI 请求中捕获；发布前仍须拒绝拓扑变化。
    pub active: Vec<UnitId>,
}

pub(in crate::desktop) struct Append {
    pub source: Source,
    pub retained: crate::pixels::Pixels,
}

impl Input {
    pub(super) fn valid(&self) -> bool {
        self.source.len() <= crate::session_document::MAX_SESSION_BYTES
            && UnitId::parse(&self.entry_id).is_some()
            && self.mouse.0.is_finite()
            && self.mouse.1.is_finite()
    }
    pub(in crate::desktop) fn prepare(
        self,
        catalog: Option<&crate::loom_handshake::Handshake>,
        cancel: &AtomicBool,
    ) -> Result<(PreparedRestore, Append), String> {
        if cancel.load(Ordering::Acquire) {
            return Err("Frozen preparation cancelled".into());
        }
        let document = crate::session_document::decode(&self.source)
            .map_err(|error| format!("Frozen source rejected: {error:?}"))?;
        let library = document.library(self.kind)?;
        let entry = library
            .find(&self.entry_id)
            .ok_or("Frozen entry not found")?;
        let copy: Source = session_library::candidate::encode_selected(
            &document, entry, self.unit, self.mouse, cancel,
        )?
        .bytes
        .into();
        let parsed = copy
            .parse()
            .map_err(|error| format!("Invalid frozen candidate: {error:?}"))?;
        let retained = session_library::retention::prepare_decoded(
            &document,
            parsed.document(),
            &self.active,
            self.kind,
            &library,
            &self.entry_id,
            cancel,
        )?;
        // 保留带预算的同一编码缓冲，不为候选再复制一次内嵌图片；冻结来源没有外部文件根目录。
        let restore = PreparedRestore::prepare_parsed(parsed, None, catalog, cancel)
            .map_err(|error| error.to_string())?;
        Ok((
            restore,
            Append {
                source: self.source,
                retained,
            },
        ))
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_frozen_input_tests.rs"]
mod tests;
