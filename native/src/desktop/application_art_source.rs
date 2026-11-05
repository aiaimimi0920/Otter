//! 子 Art 的源身份；Live 保稳定会话身份，不借同 ID 冻结后的文档替代原源 owner。
use super::*;
use crate::workspace_id::UnitId;
enum Owner {
    Document(isize, crate::desktop::configuration_stamp::Stamp),
    Live(std::sync::Arc<()>),
}
pub(in crate::desktop::application) struct Source {
    pub unit: UnitId,
    pub hwnd: HWND,
    pub port: String,
    owner: Owner,
}
impl Source {
    pub fn capture(app: &Application, unit: &UnitId) -> Result<Self> {
        let source = if let Some(doc) = app.documents.iter().find(|doc| &doc.state.unit_id == unit)
        {
            Self {
                unit: unit.clone(),
                hwnd: doc.window.0,
                port: if let Some(local) = &doc.state.local_art_id {
                    local
                        .primary_output()
                        .ok_or_else(|| Error::new(E_ABORT, "Art 来源没有图片输出端口"))?
                } else {
                    "output".into()
                },
                owner: Owner::Document(
                    &*doc.state as *const State as isize,
                    crate::desktop::configuration_stamp::Stamp::capture(&doc.state),
                ),
            }
        } else {
            if !app
                .live_sessions
                .iter()
                .any(|session| session.unit_id() == unit)
            {
                return Err(Error::new(E_ABORT, "Art 发起节点已关闭或被替换"));
            }
            return Self::capture_live(app, unit);
        };
        if !source.current(app) {
            return Err(Error::new(E_ABORT, "Art 来源已锁定、隐藏或被替换"));
        }
        Ok(source)
    }
    pub fn capture_live(app: &Application, unit: &UnitId) -> Result<Self> {
        let session = app
            .live_sessions
            .iter()
            .find(|s| s.unit_id() == unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live 发起节点不存在"))?;
        let source = Self {
            unit: unit.clone(),
            hwnd: session
                .art_dialog_owner()
                .ok_or_else(|| Error::new(E_ABORT, "Live 正在切换或关闭"))?,
            port: "output".into(),
            owner: Owner::Live(session.input_identity()),
        };
        if !source.current(app) {
            return Err(Error::new(E_ABORT, "Live 来源已变化"));
        }
        Ok(source)
    }
    pub fn current(&self, app: &Application) -> bool {
        if !app.workspace.contains(&self.unit)
            || app
                .workspace
                .group_of(&self.unit)
                .is_some_and(|group| group.hidden || group.locked)
        {
            return false;
        }
        match &self.owner {
            Owner::Document(marker, stamp) => app.documents.iter().any(|doc| {
                doc.state.unit_id == self.unit
                    && doc.window.0 == self.hwnd
                    && &*doc.state as *const State as isize == *marker
                    && stamp.matches(&doc.state)
                    && unsafe { doc.is_alive() }
                    && !app.link_owner_blocked(&self.unit)
            }),
            Owner::Live(identity) => app.live_sessions.iter().any(|session| {
                session.unit_id() == &self.unit
                    && session.is_alive()
                    && session.belongs_to(&app.workspace)
                    && std::sync::Arc::ptr_eq(identity, &session.input_identity())
                    && session.art_dialog_owner() == Some(self.hwnd)
            }),
        }
    }
    pub unsafe fn geometry(&self) -> Result<(RECT, bool)> {
        let mut rect = RECT::default();
        GetWindowRect(self.hwnd, &mut rect)?;
        Ok((rect, IsWindowVisible(self.hwnd).as_bool()))
    }
}
