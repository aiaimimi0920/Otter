//! 分组隐藏先建立每个成员的托盘恢复入口，失败回滚本次隐藏，不把窗口永久藏住。
use super::*;
use crate::{desktop::tray, workspace_id::UnitId};
impl Application {
    pub(super) unsafe fn set_group_hidden(&mut self, id: &UnitId, hidden: bool) -> Result<()> {
        self.set_group_hidden_with(
            id,
            hidden,
            |hwnd, state| tray::hide(hwnd, state),
            |hwnd, state| tray::restore_window(hwnd, state),
        )
    }
    pub(super) unsafe fn set_group_hidden_with(
        &mut self,
        id: &UnitId,
        hidden: bool,
        mut hide: impl FnMut(HWND, &State) -> Result<()>,
        mut restore: impl FnMut(HWND, &State),
    ) -> Result<()> {
        let mut group = self
            .workspace
            .groups()
            .iter()
            .find(|group| group.id == *id)
            .cloned()
            .ok_or_else(|| Error::new(E_INVALIDARG, "分组已不存在。"))?;
        let members: Vec<_> = self
            .documents
            .iter()
            .filter(|document| {
                self.workspace
                    .group_of(&document.state.unit_id)
                    .is_some_and(|group| group.id == *id)
            })
            .map(|document| (document, tray::is_hidden(&document.state)))
            .collect();
        if hidden {
            for (index, (document, _)) in members.iter().enumerate() {
                if let Err(error) = hide(document.window.0, &document.state) {
                    for (previous, was_hidden) in &members[..index] {
                        if !was_hidden {
                            restore(previous.window.0, &previous.state);
                        }
                    }
                    return Err(error);
                }
            }
        }
        group.hidden = hidden;
        if let Err(error) = self.workspace.upsert_group(group) {
            if hidden {
                for (document, was_hidden) in &members {
                    if !was_hidden {
                        restore(document.window.0, &document.state);
                    }
                }
            }
            return Err(Error::new(
                E_FAIL,
                format!("Cannot update group visibility: {error:?}"),
            ));
        }
        if !hidden {
            for (document, _) in members {
                restore(document.window.0, &document.state);
            }
        }
        Ok(())
    }
}
