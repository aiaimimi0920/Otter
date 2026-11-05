//! 单个冻结贴纸的追加发布；隐藏窗口与保留原文一起提交，不替换任何现有窗口。
use super::append::{same_space, Append};
use super::*;
use crate::{session_library::Kind, session_source::Source};
impl Application {
    /// source 必须是发起后台准备时保留的同一会话 owner，不接受仅字节相等的替代品。
    pub(in crate::desktop::application) unsafe fn prepare_frozen_append(
        &self,
        windows: Windows,
        source: Source,
        kind: Kind,
        entry_id: &str,
        space: crate::session_placement::Space,
        cancel: &AtomicBool,
    ) -> Result<Append> {
        if !windows.matches_application(self)
            || windows.documents.len() != 1
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.restored_session.as_ref().is_none_or(|record| {
                !same_space(record.space, space) || !record.original.same_storage(&source)
            })
        {
            return Err(Error::new(E_ABORT, "Frozen append source changed or busy"));
        }
        let retained = crate::session_library::retention::prepare(
            &source,
            windows.candidate.original_json(),
            self.workspace.units(),
            kind,
            entry_id,
            cancel,
        )
        .map_err(|error| Error::new(E_INVALIDARG, error))?;
        self.prepare_append(windows, source, retained, space)
    }

    pub(in crate::desktop::application) unsafe fn publish_frozen_append(
        &mut self,
        append: Append,
        visible: bool,
    ) -> Result<()> {
        self.publish_append(append, visible)
    }
}

#[path = "application_frozen_append_smoke.rs"]
pub(super) mod smoke;
