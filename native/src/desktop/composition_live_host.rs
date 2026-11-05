//! UI 线程上的 Live 呈现控制器；宿主仍拥有 HWND、消息分发和捕获授权。
use super::{notification::WindowSignal, *};
use crate::live_frames::{Consumer, Delivery};
use windows::Win32::Foundation::{LPARAM, WPARAM};

#[derive(Debug, PartialEq, Eq)]
pub enum Update {
    Ignored,
    Idle,
    Frame,
    Closed,
}

pub struct Host {
    signal: Option<WindowSignal<Texture>>,
    surface: Option<Surface>,
    pending: Option<Arc<Frame<Texture>>>,
    hwnd: HWND,
    failures: u64,
    opacity: f32,
    chrome_visible: bool,
    presented: Option<([u32; 2], u64, crate::live_viewport::Viewport)>,
}
impl Host {
    /// 不启动捕获；消费已获授权的源。须在 HWND 销毁前丢弃本对象。
    pub fn bind(hwnd: HWND, consumer: Consumer<Texture>) -> Result<Self> {
        Ok(Self {
            signal: Some(WindowSignal::bind(hwnd, consumer)?),
            surface: None,
            pending: None,
            hwnd,
            failures: 0,
            opacity: 1.0,
            chrome_visible: true,
            presented: None,
        })
    }
    /// Frame/Closed 时宿主应使窗口失效；不在消息突发期间绘制每一帧。
    pub fn receive(&mut self, message: u32, wp: WPARAM, lp: LPARAM) -> Result<Update> {
        let delivery = self
            .signal
            .as_mut()
            .and_then(|signal| signal.receive(self.hwnd, message, wp, lp));
        match delivery {
            Some(delivery) => self.accept(delivery),
            None => Ok(Update::Ignored),
        }
    }
    pub fn notification_failures(&self) -> u64 {
        self.signal
            .as_ref()
            .map_or(self.failures, WindowSignal::notification_failures)
    }
    pub fn is_closed(&self) -> bool {
        self.signal.is_none()
    }
    /// 透明度属于窗口视觉，不烘焙到帧；首帧前/暂停期间保留，重建 surface 后重新应用。
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(invalid("Invalid Live opacity"));
        }
        if let Some(surface) = self.surface.as_mut() {
            surface.set_opacity(opacity)?;
        }
        self.opacity = opacity;
        Ok(())
    }
    pub(in crate::desktop) fn set_chrome_visible(&mut self, visible: bool) {
        self.chrome_visible = visible;
    }
    /// 显式快照领取最新已投递帧（可能尚未呈现），不改变输入所用的 presented 身份。
    /// 调用方须持有整个 Arc；关闭后不得回退到先前 surface 的旧画面。
    pub fn snapshot_frame(&mut self) -> Result<Option<Arc<Frame<Texture>>>> {
        if self.refresh()? == Update::Closed {
            return Ok(None);
        }
        Ok(self
            .pending
            .clone()
            .or_else(|| self.surface.as_ref().map(|surface| surface.frame.clone())))
    }
    /// 仅返回成功提交呈现的几何；不是输入授权，也不保证 compositor 已扫描输出。
    /// 调用方传入当前客户区尺寸，拒绝 resize 后尚未重新呈现的旧坐标。
    pub fn presented_viewport(
        &self,
        size: [u32; 2],
    ) -> Option<(u64, crate::live_viewport::Viewport)> {
        self.presented
            .filter(|(presented_size, _, _)| *presented_size == size)
            .map(|(_, sequence, viewport)| (sequence, viewport))
    }
    #[cfg(test)]
    pub(in crate::desktop) fn applied_opacity_for_test(&self) -> Option<f32> {
        self.surface
            .as_ref()
            .map(|surface| surface.composition.opacity_for_test())
    }
    #[cfg(test)]
    pub(super) fn has_surface(&self) -> bool {
        self.surface.is_some()
    }
    #[cfg(test)]
    pub(in crate::desktop) fn frame_for_test(&self) -> Option<Arc<Frame<Texture>>> {
        self.surface.as_ref().map(|surface| surface.frame.clone())
    }
    /// 供宿主已有事件补偿读取。终止投递失败且之后无事件时，仍需宿主关闭策略。
    pub fn refresh(&mut self) -> Result<Update> {
        match self.signal.as_mut() {
            Some(signal) => {
                let delivery = signal.refresh();
                self.accept(delivery)
            }
            None => Ok(Update::Closed),
        }
    }
    /// 暂停输出但不取消订阅；仅保留最新帧，恢复不依赖源再次产生画面变化。
    pub fn suspend(&mut self) {
        self.presented = None;
        if let Some(surface) = self.surface.take() {
            if self.pending.is_none() {
                self.pending = Some(surface.frame.clone());
            }
            // 释放呈现链与导入位图；不提前释放共享帧的预算 owner。
            drop(surface);
        }
    }
    /// 宿主负责 BeginPaint/EndPaint。零尺寸释放呈现资源，不自发重绘。
    pub fn paint(&mut self, size: D2D_SIZE_U) -> Result<bool> {
        self.refresh()?;
        if size.width == 0 || size.height == 0 {
            self.suspend();
            return Ok(false);
        }
        let result = self.paint_inner(size);
        if result.is_err() {
            // 设备丢失/无效输入不留住 GPU owner，也不形成重绘重试风暴。
            self.close();
        }
        result
    }
    fn paint_inner(&mut self, size: D2D_SIZE_U) -> Result<bool> {
        if let Some(frame) = self.pending.take() {
            match self.surface.as_mut() {
                Some(surface) => {
                    surface.set_frame(frame)?;
                }
                None => self.surface = Some(Surface::new(self.hwnd, size, frame)?),
            }
        }
        let Some(surface) = self.surface.as_mut() else {
            return Ok(false);
        };
        surface.resize(size)?;
        surface.set_opacity(self.opacity)?;
        surface.draw_chrome(self.chrome_visible)?;
        surface.present()?;
        let destination = [size.width, size.height];
        self.presented = crate::live_viewport::Viewport::fit(
            [surface.frame.resource.width, surface.frame.resource.height],
            destination,
        )
        .map(|viewport| (destination, surface.frame.sequence, viewport));
        Ok(true)
    }
    fn accept(
        &mut self,
        delivery: std::result::Result<Option<Delivery<Texture>>, crate::live_frames::Error>,
    ) -> Result<Update> {
        match delivery {
            Ok(Some(delivery)) => {
                self.pending = Some(delivery.frame);
                Ok(Update::Frame)
            }
            Ok(None) => Ok(Update::Idle),
            Err(crate::live_frames::Error::Closed) => {
                self.close();
                Ok(Update::Closed)
            }
            Err(error) => {
                self.close();
                Err(invalid(&format!("Live consumer failed: {error:?}")))
            }
        }
    }
    /// 先取消订阅，再释放待绘帧和 GPU surface；最后消费者离开会唤醒采集退出。
    pub fn close(&mut self) {
        self.failures = self.notification_failures();
        self.signal = None;
        self.pending = None;
        self.surface = None;
        self.presented = None;
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[path = "composition_live_host_tests.rs"]
mod tests;
