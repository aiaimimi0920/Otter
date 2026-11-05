//! OpenArt 的独立节点事务：候选窗口和拓扑先准备，授权启动成功后一起发布，不转换源截图。
use super::*;
use crate::{device_authorization_config::Config, workspace::Link};
impl Application {
    pub(super) fn open_local_art(
        &mut self,
        origin: &str,
        art: String,
        node: &str,
        config: Config,
    ) -> std::result::Result<(), String> {
        unsafe { self.open_local_art_inner(origin, art, node, config, false, |_| true) }
            .map_err(|error| error.message())
    }
    pub(super) fn open_local_art_child(
        &mut self,
        origin: &str,
        art: String,
        node: &str,
        config: Config,
        current: impl Fn(&Self) -> bool,
    ) -> Result<()> {
        unsafe { self.open_local_art_inner(origin, art, node, config, true, current) }
    }
    unsafe fn open_local_art_inner(
        &mut self,
        origin: &str,
        art: String,
        node: &str,
        config: Config,
        always_create: bool,
        current: impl Fn(&Self) -> bool,
    ) -> Result<()> {
        let fail = |message: &str| Error::new(E_ABORT, message);
        if self.modals.blocked()
            || self.session_end.paused()
            || self.exit_requested.get()
            || self.restore_job.is_some()
            || self.save_job.is_some()
            || config.origin() != origin
            || !current(self)
        {
            return Err(fail("工作区正在切换、保存或退出，Art 未启动"));
        }
        let source = self.documents.iter().find(|doc| {
            doc.state.unit_id.as_str() == node
                && doc.is_alive()
                && self.workspace.contains(&doc.state.unit_id)
        });
        if let Some(source) = source {
            let selection = source.state.selected_art.borrow();
            let same_art = selection
                .as_ref()
                .map(|s| s.parameters.art_id())
                .or(source.state.local_art_id.as_deref())
                .or(match &source.state.restored_naming_art {
                    super::super::naming_context::RestoredArtId::Value(id) => Some(id.as_str()),
                    _ => None,
                })
                == Some(art.as_str());
            drop(selection);
            if !always_create && source.state.is_art() && same_art {
                return self
                    .start_local_art(origin, art, node, config)
                    .map_err(|message| fail(&message));
            }
            if source.state.group_locked.get() || source.state.file_dialog.get() {
                return Err(fail("Art 发起节点已锁定或正在编辑"));
            }
        }
        let unit =
            crate::workspace_id::UnitId::parse(node).ok_or_else(|| fail("Art 来源身份无效"))?;
        let source = super::add_art::source::Source::capture(self, &unit)?;
        if art.is_empty() || art.len() > 256 || art.chars().any(char::is_control) {
            return Err(fail("Art 身份无效"));
        }
        let now = super::super::surface_windows::now_ms().map_err(|_| fail("无法读取授权时钟"))?;
        let lease = match self.authorization_owner.lease(origin, now) {
            Ok(lease) => lease,
            Err(crate::device_authorization_owner::Error::Credential)
                if self.authorization_owner.can_resume_local(origin) =>
            {
                self.art_issuers
                    .pending_local(origin)
                    .map_err(|_| fail("请先获取有效设备授权"))?
            }
            Err(_) => return Err(fail("请先获取有效设备授权")),
        };
        let lease = std::sync::Arc::new(lease);
        let guard = restore_batch::guard::Guard::capture(self)?;
        let (rect, visible) = source.geometry()?;
        let mut state = State::new(None, true)?;
        state.local_art_id = Some(art.clone().into());
        let unit = state.unit_id.clone();
        let mut workspace = self.workspace.clone();
        workspace
            .add_units([unit.clone()])
            .map_err(|_| fail("工作区节点容量不足"))?;
        let link = Link {
            id: super::super::document_state::new_unit_id()?,
            from_unit: source.unit.clone(),
            from_port: source.port.clone(),
            to_unit: unit,
            to_port: "input_image".into(),
        };
        workspace
            .connect(link.clone())
            .map_err(|_| fail("无法创建 Art 输入连接"))?;
        *state
            .local_art_id
            .as_ref()
            .unwrap()
            .pending_input
            .borrow_mut() = Some(link);
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::from(E_OUTOFMEMORY))?;
        self.capture_windows
            .borrow_mut()
            .try_reserve(2)
            .map_err(|_| Error::from(E_OUTOFMEMORY))?;
        let document = self.prepare_window_at(
            state,
            visible,
            crate::pin_geometry::PinRect {
                x: rect.left.saturating_add(32).min(i32::MAX - 320),
                y: rect.top.saturating_add(32).min(i32::MAX - 240),
                width: 320,
                height: 240,
            },
        )?;
        // 建窗/监听可能重入。最终检查后直到发布没有可重入 Win32 调用。
        if !guard.matches(self)
            || self.modals.blocked()
            || self.session_end.paused()
            || self.exit_requested.get()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || !current(self)
            || !source.current(self)
        {
            return Err(fail("Art 发起节点已变化"));
        }
        self.local_arts
            .start(
                &document,
                &workspace,
                art,
                lease,
                &mut self.art_issuers,
                config,
            )
            .map_err(fail)?;
        let hwnd = document.window.0;
        self.workspace = workspace;
        self.capture_windows
            .borrow_mut()
            .extend([hwnd, document.panel.0]);
        self.documents.push(document);
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
}
