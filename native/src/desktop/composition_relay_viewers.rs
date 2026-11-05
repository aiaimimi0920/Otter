//! 观看身份与待加入围栏复用原Session；已打开窗口不依赖发起面板继续存活。
use super::*;
use crate::surface_live_binding::Binding;
use std::sync::OnceLock;
#[path = "composition_relay_viewer_host.rs"]
mod host;
pub(crate) use host::ViewerHost;

pub struct ViewerKey {
    origin: String,
    session: String,
}
impl ViewerKey {
    pub(super) fn new(origin: &str, session: &str) -> Self {
        Self {
            origin: origin.into(),
            session: session.into(),
        }
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn session(&self) -> &str {
        &self.session
    }
}
pub struct ViewerState {
    pub key: Arc<ViewerKey>,
    pub stopping: bool,
    pub shown: bool,
}
/// 生命周期revision不属于附件身份；同一附件的正常ACK不能取消已发起的加入。
pub(crate) fn same_binding(left: &Binding, right: &Binding) -> bool {
    left.matches(
        right.origin(),
        right.device(),
        right.node(),
        right.generation(),
    ) && left.instance() == right.instance()
        && left.attachment() == right.attachment()
}
impl Sessions {
    pub(in crate::desktop) fn take_local_triggers(
        &self,
    ) -> Vec<crate::art_live_trigger::ticket::Ticket> {
        self.entries
            .iter()
            .filter_map(|entry| entry.session.take_local_trigger())
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_viewer_control(&self, key: &Arc<ViewerKey>) -> bool {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .is_some_and(|e| e.session.test_disconnect_control())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_control(&self, key: &Arc<ViewerKey>) -> Option<(bool, u64)> {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .map(|e| e.session.test_control())
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_viewer_media(&self, key: &Arc<ViewerKey>) -> bool {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .is_some_and(|e| e.session.test_disconnect_media())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_observation(
        &self,
        key: &Arc<ViewerKey>,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        windows::Win32::Foundation::HWND,
    )> {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .and_then(|e| e.session.test_observation())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_input(
        &self,
        key: &Arc<ViewerKey>,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        crate::live_relay_control_runtime::interaction::Status,
    )> {
        self.entries
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
            .and_then(|entry| entry.session.test_input())
    }
    #[cfg(test)]
    pub(crate) fn test_frame(
        &self,
        key: &Arc<ViewerKey>,
    ) -> windows::core::Result<Option<(crate::live_relay_frame::Metadata, [u8; 4])>> {
        self.entries
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
            .map(|entry| entry.session.test_frame())
            .transpose()
            .map(Option::flatten)
    }
    pub fn viewers(&self) -> Vec<ViewerState> {
        self.entries
            .iter()
            .map(|entry| ViewerState {
                key: entry.key.clone(),
                stopping: entry.session.stopping(),
                shown: entry.session.has_window(),
            })
            .collect()
    }
    /// 同设备只能有一份同源会话观看；停止但尚未join的owner也继续占身份和四槽。
    pub fn start_viewing(
        &mut self,
        config: Config,
        lease: Lease,
        target: Target,
        binding: Binding,
    ) -> io::Result<()> {
        if self
            .entries
            .iter()
            .any(|entry| entry.key.origin == config.origin() && entry.key.session == target.session)
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        if binding.origin() != config.origin()
            || binding.device() != lease.device_id()
            || binding.instance() != target.surface
            || binding.attachment() != target.attachment
            || target
                .selected
                .as_ref()
                .is_none_or(|id| id.session() != target.session)
            || target.expected.is_some()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // 进程共享一份原协议最大编码帧预算，不按窗口复制配额；解码仍走原Pixels全局预算。
        self.start(config, lease, target, encoded_budget(), read_only)?;
        self.entries
            .last_mut()
            .expect("successful start inserts entry")
            .binding = Some(ViewerHost::Surface(binding));
        Ok(())
    }
    pub(crate) fn start_local_art_viewing(
        &mut self,
        config: Config,
        lease: Lease,
        target: crate::live_relay_control_runtime::LocalArtTarget,
    ) -> io::Result<()> {
        if self
            .entries
            .iter()
            .any(|entry| entry.key.origin == config.origin() && entry.key.session == target.session)
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        let binding = target.proof.binding();
        if binding.origin != config.origin()
            || binding.device_id != lease.device_id()
            || target.selected.session() != target.session
            || target.proof.validate().is_err()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let host = ViewerHost::LocalArt(target.proof.clone());
        self.insert_session(config.origin().into(), target.session.clone(), || {
            crate::live_relay_control_runtime::Runtime::start_local_art_viewer(
                config,
                lease,
                target,
                encoded_budget(),
                read_only,
            )
            .map(Session::from_runtime)
        })?;
        self.entries
            .last_mut()
            .expect("successful start inserts entry")
            .binding = Some(host);
        Ok(())
    }
    pub fn stop_viewer(&mut self, key: &Arc<ViewerKey>) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
        else {
            return false;
        };
        entry.session.stop();
        true
    }
    pub fn cancel_viewers(&mut self) {
        for entry in &mut self.entries {
            entry.session.stop();
        }
    }
    /// 必须在poll领取Ready前调用；旧附件迟到成功不建窗。已Ready的独立观看由原授权结束。
    pub(crate) fn cancel_pending_viewers(&mut self, mut current: impl FnMut(&ViewerHost) -> bool) {
        for entry in &mut self.entries {
            if !entry.session.joined()
                && entry
                    .binding
                    .as_ref()
                    .is_some_and(|binding| !current(binding))
            {
                entry.session.stop();
            }
        }
    }
}
pub(super) fn encoded_budget() -> Arc<Budget> {
    static ENCODED: OnceLock<Arc<Budget>> = OnceLock::new();
    ENCODED
        .get_or_init(|| {
            Budget::new(
                crate::live_relay_frame::MAX_FRAME_BYTES + crate::live_relay_frame::HEADER_LEN,
            )
        })
        .clone()
}
/// 原Owner先事务校验并发布Business/状态；此只读适配器不执行source输入或Surface动作。
/// 未知业务不默默ACK；已知观察/审计由原Host保留，后续UI可读同一快照。
fn read_only(events: &[ControlEvent]) -> Result<(), ()> {
    // trigger_condition已由Loom登记，input_event只由source执行；viewer不二次执行。
    if events.iter().all(|event| {
        matches!(
            event.kind(),
            "session_state" | "observation" | "trigger_event" | "input_event" | "trigger_condition"
        )
    }) {
        Ok(())
    } else {
        Err(())
    }
}
#[cfg(test)]
#[path = "composition_relay_viewers_tests.rs"]
mod tests;
