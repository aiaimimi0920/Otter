//! 窗口恢复事务独占退休 Target；普通 discard/换图不能丢失退休门禁或授权新关联。
use super::{Canvas, Target};
use crate::desktop::composition_retirement::{Stage, Status};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_FAIL},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::desktop) enum Destination {
    Automatic,
    #[default]
    Compatibility,
    Software,
}

#[derive(Default)]
pub(super) struct Recovery {
    pub(super) target: Option<Target>,
    pub(super) status: Option<Status>,
    destination: Destination,
}

impl Canvas {
    pub(in crate::desktop) fn software_selected(&self) -> bool {
        self.software_renderer
    }
    pub(in crate::desktop) fn render_choice(&self) -> Destination {
        if self.software_renderer {
            Destination::Software
        } else if self.compatibility_renderer {
            Destination::Compatibility
        } else {
            Destination::Automatic
        }
    }
    pub(in crate::desktop) fn needs_gpu_style(&self) -> bool {
        self.target.is_none() && !self.software_renderer
    }
    /// 用户选择不能清除创建期拒绝或未确认退休；终态只允许重建已知无合成图的 owner。
    pub(in crate::desktop) fn can_request_render(&self) -> bool {
        if self.creation_blocked() || self.recovery.status == Some(Status::Pending) {
            return false;
        }
        self.recovery.status.is_none()
            || (self.software_renderer && self.recovery.target.is_none())
            || (self.recovery.target.is_none() && self.creation_admission.retryable_failure())
            || self
                .recovery
                .target
                .as_ref()
                .is_some_and(|target| target.surface.is_software())
    }
    /// 返回 true 表示仍须窗口计时器完成退休；成功接纳请求不代表已呈现，更不能清原错误。
    pub(in crate::desktop) fn request_render(&mut self, destination: Destination) -> Result<bool> {
        if !self.can_request_render() {
            return Err(Error::new(
                E_ABORT,
                "Render association has not authorized a manual retry",
            ));
        }
        if self.recovery.status.is_some() {
            self.recovery.target.take();
            self.recovery.status = None;
        }
        if self.target.as_ref().is_some_and(|target| {
            matches!(
                target.surface,
                super::RenderSurface::D3d12(_) | super::RenderSurface::D3d11(_)
            )
        }) {
            self.begin_recovery(destination);
            return Ok(true);
        }
        self.discard_device();
        self.select_render(destination);
        Ok(false)
    }
    fn select_render(&mut self, destination: Destination) {
        self.compatibility_renderer = destination != Destination::Automatic;
        self.software_renderer = destination == Destination::Software;
    }
    pub(in crate::desktop) fn creation_blocked(&self) -> bool {
        self.creation_admission.blocked()
    }
    pub(in crate::desktop) fn render_suspended(&self) -> bool {
        self.recovery.status.is_some()
    }
    pub(super) fn ensure_render_admitted(&self) -> Result<()> {
        if self.render_suspended() {
            Err(Error::new(
                E_ABORT,
                "Render recovery has not authorized a new frame",
            ))
        } else {
            Ok(())
        }
    }
    /// 显式自检退休不启动窗口恢复；调用者仍负责停止绘制并保留 Canvas。
    pub(in crate::desktop) fn retire_composition(
        &mut self,
    ) -> Option<crate::desktop::composition_retirement::Status> {
        self.target
            .as_mut()
            .and_then(|target| target.surface.retire_composition())
    }
    pub(in crate::desktop) fn recover_render(&mut self) -> bool {
        if self.render_suspended() || self.software_renderer {
            return false;
        }
        let Some(target) = &self.target else {
            return false;
        };
        let destination = if !self.compatibility_renderer && target.surface.requires_compatibility()
        {
            Destination::Compatibility
        } else if target.surface.requires_software() {
            Destination::Software
        } else {
            return false;
        };
        self.begin_recovery(destination);
        true
    }
    pub(in crate::desktop) fn recover_render_after(
        &mut self,
        _code: windows::core::HRESULT,
    ) -> bool {
        // HRESULT 本身不是 DComp 退休凭证；两种 GPU 均由实际 owner 的失败状态发起恢复。
        self.recover_render()
    }
    fn begin_recovery(&mut self, destination: Destination) {
        // 完整 owner 移交后，最小化、换源及普通 discard 均只触及活动 target。
        self.recovery.target = self.target.take();
        self.recovery.destination = destination;
        self.recovery.status = Some(Status::Pending);
    }
    /// 无 DComp 关联的 CPU target 保留一次重建；两种 GPU 都不能借此绕过退休。
    pub(in crate::desktop) fn retry_render_without_composition(&mut self) -> bool {
        if self.render_suspended()
            || self.target.as_ref().is_some_and(|target| {
                matches!(
                    target.surface,
                    super::RenderSurface::D3d12(_) | super::RenderSurface::D3d11(_)
                )
            })
        {
            return false;
        }
        self.discard_device();
        true
    }
    pub(in crate::desktop) fn poll_render_recovery(&mut self) -> Option<Status> {
        if self.recovery.status != Some(Status::Pending) {
            return self.recovery.status;
        }
        let status = self
            .recovery
            .target
            .as_mut()
            .and_then(|target| target.surface.retire_composition())
            .unwrap_or(Status::Failed {
                stage: Stage::Worker,
                code: E_FAIL,
            });
        self.resolve_recovery(status);
        Some(status)
    }
    fn resolve_recovery(&mut self, status: Status) {
        // 窗口终态不可被晚到成功解锁，也不能重复发布兼容候选。
        if self.recovery.status != Some(Status::Pending) {
            return;
        }
        if status == Status::Complete {
            // Complete 仅确认 DComp。旧 owner 的 Drop 仍独立执行 GPU drain/保留策略。
            self.recovery.target.take();
            self.select_render(self.recovery.destination);
            self.recovery.status = None;
        } else {
            self.recovery.status = Some(status);
        }
    }
    /// 调度失败或不可恢复的绘制错误：保文档、停止重试，不把失败翻译成关窗。
    pub(in crate::desktop) fn suspend_render(&mut self) {
        if self.recovery.target.is_none() {
            self.recovery.target = self.target.take();
        }
        if let Some(target) = self.recovery.target.as_mut() {
            let _ = target.surface.retire_composition();
        }
        self.recovery.status = Some(Status::Failed {
            stage: Stage::Worker,
            code: E_ABORT,
        });
    }
    pub(in crate::desktop) fn close_render(&mut self) {
        // HWND 销毁后不再接续；未确认图由后端保留型 Drop 处理，绝不 join 系统等待线程。
        self.recovery.status = Some(Status::Failed {
            stage: Stage::Worker,
            code: E_ABORT,
        });
        self.recovery.target.take();
        self.discard_device();
    }
}

#[cfg(test)]
#[path = "canvas_render_recovery_tests.rs"]
mod tests;
