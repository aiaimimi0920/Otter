//! 单贴纸追加的共同窗口事务；冻结库与投送共享唯一发布 owner，不替换现有工作区。
use super::*;
use crate::session_source::Source;
pub(in crate::desktop::application) struct Append {
    pub(super) windows: Windows,
    source: Source,
    retained: crate::pixels::Pixels,
    space: crate::session_placement::Space,
}
impl Application {
    pub(in crate::desktop::application) unsafe fn prepare_append(
        &self,
        windows: Windows,
        source: Source,
        retained: crate::pixels::Pixels,
        space: crate::session_placement::Space,
    ) -> Result<Append> {
        if !windows.matches_application(self)
            || windows.documents.len() != 1
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.restored_session.as_ref().is_none_or(|record| {
                !same_space(record.space, space) || !record.original.same_storage(&source)
            })
        {
            return Err(Error::new(E_ABORT, "Append source changed or busy"));
        }
        Ok(Append {
            windows,
            source,
            retained,
            space,
        })
    }
    pub(in crate::desktop::application) unsafe fn publish_append(
        &mut self,
        mut append: Append,
        visible: bool,
    ) -> Result<()> {
        let valid = |app: &Application| {
            append.windows.matches_application(app)
                && app.save_job.is_none()
                && app.restore_job.is_none()
                && app.restored_session.as_ref().is_some_and(|record| {
                    same_space(record.space, append.space)
                        && record.original.same_storage(&append.source)
                })
                && append
                    .windows
                    .documents
                    .iter()
                    .all(|doc| !app.workspace.contains(&doc.state.unit_id))
        };
        if !valid(self) {
            return Err(Error::new(E_ABORT, "Append is stale"));
        }
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve appended window"))?;
        self.capture_windows
            .borrow_mut()
            .try_reserve(2)
            .map_err(|_| {
                Error::new(
                    E_OUTOFMEMORY,
                    "Cannot reserve appended capture registration",
                )
            })?;
        self.restored_session
            .as_mut()
            .unwrap()
            .annotation_ids
            .try_reserve(append.windows.annotation_ids.len())
            .map_err(|_| {
                Error::new(
                    E_OUTOFMEMORY,
                    "Cannot reserve appended annotation identities",
                )
            })?;
        if visible {
            append.windows.documents[0].observe_priority()?;
        }
        // 系统监听安装可能重入；最终复核后不调用 Win32、不派发消息，原文和窗口一起接管。
        if !valid(self) {
            return Err(Error::new(E_ABORT, "Append changed during publication"));
        }
        let document = append.windows.documents.pop().unwrap();
        self.workspace
            .add_units([document.state.unit_id.clone()])
            .map_err(|error| Error::new(E_FAIL, format!("Cannot append identity: {error:?}")))?;
        let hwnd = document.window.0;
        let panel = document.panel.0;
        let pinned = document.state.canvas.borrow().pinned;
        self.documents.push(document);
        self.capture_windows.borrow_mut().extend([hwnd, panel]);
        let record = self.restored_session.as_mut().unwrap();
        record.edit_original(append.retained.into());
        record
            .annotation_ids
            .retain(|(unit, _)| self.workspace.contains(unit));
        record
            .annotation_ids
            .append(&mut append.windows.annotation_ids);
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            if !pinned {
                let _ = ShowWindow(panel, SW_SHOWNOACTIVATE);
            }
        }
        Ok(())
    }
}
pub(in crate::desktop::application) fn same_space(
    current: Option<crate::session_placement::Space>,
    expected: crate::session_placement::Space,
) -> bool {
    current.is_some_and(|space| {
        space.origin == expected.origin && space.scale.to_bits() == expected.scale.to_bits()
    })
}
