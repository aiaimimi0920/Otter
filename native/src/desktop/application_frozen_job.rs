//! 冻结恢复/参考复制复用应用单任务恢复 owner；整个后台周期持有同一来源与坐标凭证。
use super::*;
use crate::{session_library::Kind, session_source::Source, workspace_id::UnitId};

pub(super) struct Selection {
    pub source: Source,
    pub kind: Kind,
    pub entry_id: String,
}
impl Application {
    /// 在菜单模态范围退出后调用；source 是菜单列出条目时保留的同一会话 owner。
    /// 此入口不授权外部文件。嵌入图片可恢复，其他资源仍由既有资源准入拒绝。
    pub(in crate::desktop::application) unsafe fn request_frozen(
        &mut self,
        source: Source,
        kind: Kind,
        entry_id: String,
        physical_mouse: POINT,
    ) -> Result<UnitId> {
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "Missing frozen session source"))?;
        if !record.original.same_storage(&source) || UnitId::parse(&entry_id).is_none() {
            return Err(Error::new(
                E_ABORT,
                "Frozen menu source is stale or invalid",
            ));
        }
        let space = crate::session_coordinates::select(record.space, None)
            .map_err(|message| Error::new(E_INVALIDARG, message))?;
        let unit = crate::desktop::document_state::new_unit_id()?;
        let mouse = (
            (physical_mouse.x as f64 - space.origin.0 as f64) / space.scale,
            (physical_mouse.y as f64 - space.origin.1 as f64) / space.scale,
        );
        let input = session_job::Input::Frozen(session_job::frozen::Input {
            source: source.clone(),
            kind,
            entry_id: entry_id.clone(),
            unit: unit.clone(),
            mouse,
        });
        // 复用 save/restore 互斥、起始 guard、目录快照、唤醒、取消与退出回收。
        self.start_restore(
            input,
            None,
            Some(space),
            Some(Selection {
                source,
                kind,
                entry_id,
            }),
        )?;
        Ok(unit)
    }
}

#[path = "application_frozen_job_smoke.rs"]
pub(super) mod smoke;
