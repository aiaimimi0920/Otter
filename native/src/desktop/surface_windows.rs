//! 有界授权窗口集合；终态保留附件 owner，但不再参与等待或截止计算。
use super::surface_authorized_window::AuthorizedWindow;
use std::{
    os::windows::io::BorrowedHandle,
    time::{Duration, Instant},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_FAIL,
};
const MAX_WINDOWS: usize = 4;
#[path = "surface_failed_admission.rs"]
mod failed;
#[path = "surface_windows_images.rs"]
mod images;
#[path = "surface_windows_shutdown.rs"]
mod shutdown;
pub use failed::FailedAdmission;
pub(crate) use images::StartImageError;
#[derive(Default)]
pub struct Windows {
    entries: Vec<AuthorizedWindow>,
    failures: Vec<FailedAdmission>,
    image_cursor: usize,
}
impl Windows {
    pub fn status(&self) -> Option<String> {
        if let Some(failure) = self.failures.last() {
            return Some(format!("Surface 未打开：{}", failure.reason));
        }
        if let Some(error) = self.entries.iter().find_map(|w| w.activation_error()) {
            return Some(format!("Surface 操作未完成：{error}"));
        }
        if let Some(error) = self.entries.iter().find_map(|w| w.image_error()) {
            return Some(format!("Surface 图片未完成：{error}"));
        }
        let shown = self.entries.iter().filter(|w| w.is_shown()).count();
        if shown > 0 {
            return Some(format!("已打开 {shown} 个 Surface 窗口"));
        }
        if self.entries.iter().any(|w| w.is_alive()) {
            return Some("Surface 正在激活".into());
        }
        if !self.entries.is_empty()
            && self.entries.iter().all(|window| {
                window.lifecycle().is_some_and(|owner| {
                    !owner.uncertain()
                        && owner.binding().lifecycle()
                            == crate::surface_live_binding::Lifecycle::Inactive
                })
            })
        {
            return Some("Surface 窗口已停用".into());
        }
        (!self.entries.is_empty()).then(|| "Surface 窗口已关闭，附件状态待处理".into())
    }
    pub fn can_mount(&self, origin: &str, node: &str) -> bool {
        self.entries.len() + self.failures.len() < MAX_WINDOWS
            && !self
                .entries
                .iter()
                .any(|w| w.binding().origin() == origin && w.binding().node() == node)
            && !self
                .failures
                .iter()
                .any(|f| f.outcome.origin == origin && f.outcome.request.node() == node)
    }
    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut AuthorizedWindow> {
        self.entries.iter_mut()
    }
    /// 只保留当前活窗口/场景的资源请求；生命周期版本与场景版本分别比较。
    pub(super) fn retains_resource(
        &self,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
    ) -> bool {
        self.entries.iter().any(|window| {
            window.is_alive()
                && window.binding() == binding
                && window.snapshot_revision() == revision
        })
    }
    /// 事件按精确 pending event 路由；接纳后调用方必须保存返回收据。
    pub fn accept_batch(
        &mut self,
        outcome: crate::surface_event_batch_runtime::Outcome,
        now_ms: u64,
    ) -> std::result::Result<
        super::surface_authorized_window::BatchReceipt,
        Box<crate::surface_event_batch_runtime::Outcome>,
    > {
        let mut pending = Box::new(outcome);
        for window in &mut self.entries {
            match window.accept_batch(*pending, now_ms) {
                Ok(receipt) => return Ok(receipt),
                Err(returned) => pending = returned,
            }
        }
        Err(pending)
    }
    /// 事件按精确 pending event 路由；接纳后调用方必须保存返回收据。
    pub fn accept_event(
        &mut self,
        outcome: crate::surface_event_runtime::Outcome,
        now_ms: u64,
    ) -> std::result::Result<
        super::surface_authorized_window::EventReceipt,
        Box<crate::surface_event_runtime::Outcome>,
    > {
        let mut pending = Box::new(outcome);
        for window in &mut self.entries {
            match window.accept_event(*pending, now_ms) {
                Ok(receipt) => return Ok(receipt),
                Err(returned) => pending = returned,
            }
        }
        Err(pending)
    }
    /// 未匹配或身份/版本拒收时归还整个结果，由原任务集合继续持有。
    pub fn accept_activation(
        &mut self,
        outcome: crate::surface_lifecycle_runtime::Outcome,
        now_ms: u64,
        visible: bool,
    ) -> std::result::Result<(), Box<crate::surface_lifecycle_runtime::Outcome>> {
        if matches!(&outcome.result, Ok(Some(_))) {
            return Err(Box::new(outcome));
        }
        let binding = outcome.owner.binding();
        let Some(window) = self.entries.iter_mut().find(|window| {
            let expected = window.binding();
            window.lifecycle().is_none()
                && expected.origin() == binding.origin()
                && expected.instance() == binding.instance()
                && expected.attachment() == binding.attachment()
        }) else {
            let mut outcome = Box::new(outcome);
            for failure in &mut self.failures {
                match failure.accept(*outcome) {
                    Ok(()) => return Ok(()),
                    Err(returned) => outcome = returned,
                }
            }
            return Err(outcome);
        };
        let crate::surface_lifecycle_runtime::Outcome { owner, result } = outcome;
        if let Err(owner) = window.finish_activation(owner) {
            return Err(Box::new(crate::surface_lifecycle_runtime::Outcome {
                owner: *owner,
                result,
            }));
        }
        match result {
            Ok(None) => {
                let refresh = window.refresh(now_ms);
                if let Err(error) = refresh {
                    window.record_activation_error(error.to_string());
                } else if visible
                    && window.is_alive()
                    && window
                        .lifecycle()
                        .is_some_and(|owner| owner.confirmed_active())
                {
                    if let Err(error) = window.show(now_ms) {
                        window.record_activation_error(error.to_string());
                    }
                }
            }
            Ok(Some(_)) => unreachable!("recovery result is returned before owner transfer"),
            Err(error) => window.record_activation_error(format!("生命周期操作未完成：{error:?}")),
        }
        Ok(())
    }
    /// 容量失败归还原 owner，调用方不能丢弃附件恢复证据。
    pub fn insert(
        &mut self,
        window: AuthorizedWindow,
    ) -> std::result::Result<(), Box<AuthorizedWindow>> {
        if self.entries.len() + self.failures.len() >= MAX_WINDOWS
            || self.entries.try_reserve(1).is_err()
        {
            return Err(Box::new(window));
        }
        self.entries.push(window);
        Ok(())
    }
    /// 建窗保持隐藏；失败记录也占容量，防止错误结果无限积累或每轮重复建窗。
    pub fn admit(
        &mut self,
        outcome: crate::surface_attach_runtime::Outcome,
        authorize: impl FnOnce(
            &crate::surface_attach_runtime::Outcome,
        )
            -> std::result::Result<crate::device_authorization_lease::Lease, String>,
        now_ms: u64,
    ) -> std::result::Result<(), Box<crate::surface_attach_runtime::Outcome>> {
        if self.entries.len() + self.failures.len() >= MAX_WINDOWS
            || self.entries.try_reserve(1).is_err()
            || self.failures.try_reserve(1).is_err()
        {
            return Err(Box::new(outcome));
        }
        let duplicate = outcome.binding.as_ref().is_some_and(|binding| {
            self.entries.iter().any(|window| {
                let current = window.binding();
                current.origin() == binding.origin()
                    && current.instance() == binding.instance()
                    && current.attachment() == binding.attachment()
            }) || self.failures.iter().any(|failure| {
                failure.outcome.binding.as_ref().is_some_and(|current| {
                    current.origin() == binding.origin()
                        && current.instance() == binding.instance()
                        && current.attachment() == binding.attachment()
                })
            })
        });
        let result = if duplicate {
            Err("附件已有持有记录，保留重复挂载结果".to_owned())
        } else if outcome.result.is_err() || outcome.uncertain || outcome.binding.is_none() {
            Err("挂载结果未确认，保留恢复证据".to_owned())
        } else {
            authorize(&outcome).and_then(|lease| {
                AuthorizedWindow::new(&outcome, lease, now_ms, [960, 640])
                    .map_err(|error| format!("Surface 场景未接纳：{error:?}"))
            })
        };
        match result {
            Ok(window) => self.entries.push(window),
            Err(reason) => self
                .failures
                .push(FailedAdmission::new(outcome, reason, duplicate)),
        }
        Ok(())
    }
    pub fn take_failure(&mut self) -> Option<FailedAdmission> {
        let index = self
            .failures
            .iter()
            .rposition(|failure| !failure.pending())?;
        Some(self.failures.remove(index))
    }
    pub(super) fn failures_mut(&mut self) -> impl Iterator<Item = &mut FailedAdmission> {
        self.failures.iter_mut()
    }
    pub fn poll(&mut self, now_ms: u64) -> Result<()> {
        let mut first = None;
        for window in &mut self.entries {
            if let Err(error) = window.refresh(now_ms) {
                first.get_or_insert(error);
            }
        }
        first.map_or(Ok(()), Err)
    }
    pub fn wait_handles(&self) -> Vec<BorrowedHandle<'_>> {
        self.entries
            .iter()
            .filter(|w| w.is_alive())
            .map(|w| w.revocation_handle())
            .chain(self.javascript_wait_handles())
            .collect()
    }
    pub(crate) fn wait_budget(&self) -> usize {
        self.entries.iter().map(AuthorizedWindow::wait_budget).sum()
    }
    pub(crate) fn javascript_wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.entries
            .iter()
            .flat_map(|window| window.javascript_wait_handles())
    }
    pub fn deadline(&self, now_ms: u64) -> Result<Option<Instant>> {
        self.entries
            .iter()
            .filter(|w| w.is_alive())
            .map(|w| w.wake_at_ms())
            .min()
            .map(|expires| {
                Instant::now()
                    .checked_add(Duration::from_millis(expires.saturating_sub(now_ms)))
                    .ok_or_else(|| Error::new(E_FAIL, "Surface authorization deadline overflow"))
            })
            .transpose()
    }
    pub fn take_closed(&mut self) -> Option<AuthorizedWindow> {
        let index = self
            .entries
            .iter()
            .position(|w| !w.is_alive() && w.lifecycle().is_some() && w.javascript_finished())?;
        Some(self.entries.swap_remove(index))
    }
    /// 本地退出不冒充远端销毁；附件记录留在集合供调用方收割。
    pub fn close_all(&self) -> Result<()> {
        let mut first = None;
        for window in &self.entries {
            if let Err(error) = window.close() {
                first.get_or_insert(error);
            }
        }
        first.map_or(Ok(()), Err)
    }
}
pub(super) fn now_ms() -> Result<u64> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::new(E_FAIL, "Invalid authorization clock"))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|_| Error::new(E_FAIL, "Authorization clock overflow"))
}
