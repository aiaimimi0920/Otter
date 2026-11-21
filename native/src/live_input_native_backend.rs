//! 普通应用的采集坐标及同步 motion 入口；浏览器、远程与现有 modal 保留原路径。
use super::*;
impl Backend {
    pub(super) fn ignore_inactive_motion(&mut self, event: Event) -> bool {
        // 普通应用打开模态框时会禁用原窗口。只消费无边沿的新指针输入，
        // 不向正在换窗的 root 发同步 motion，也不丢弃原 held 的 up/cleanup。
        let ignore = self.local
            && !crate::live_browser_scroll::eligible(&self.root)
            && matches!(
                event,
                Event::Move { .. } | Event::Hover { .. } | Event::Wheel { .. }
            )
            && !unsafe {
                windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(self.root.hwnd())
            }
            .as_bool();
        if ignore {
            self.last_delivery = live_input::Delivery::IgnoredWhileModal;
        }
        ignore
    }
    pub(crate) fn with_hover(mut self, owner: Option<crate::live_hover_owner::Control>) -> Self {
        self.hover_owner = owner;
        self
    }
    pub(super) fn check_control(&mut self) -> Result<(), Error> {
        if let Some(control) = self.hover_owner {
            if !self.local {
                return Err(Error::InvalidDelivery);
            }
            if !control.identity().valid() {
                return Err(Error::NativePopup(
                    "Live controller identity changed".into(),
                ));
            }
            if !control.valid() {
                // 撤销只可静默结束确定未发送的新操作，不能遮住原在途 modal 的未知结果。
                self.modal.check_revoked_control().map_err(Error::Message)?;
                return Err(Error::ControlRevoked);
            }
        }
        Ok(())
    }
    pub(super) fn map_point(&self, target: &Destination, point: [f64; 2]) -> Result<Mapped, Error> {
        let map = if self.local && !crate::live_browser_scroll::eligible(&self.root) {
            coordinates::map_capture
        } else {
            coordinates::map
        };
        map(&self.root, target, point, self.region).map_err(Error::Coordinates)
    }
    pub(super) fn prepare_native(&mut self) -> Result<(), Error> {
        self.check_control()?;
        if let Some(control) = self.hover_owner {
            let owner = control.identity();
            if self.hover_binding.is_none() {
                self.hover_binding = Some(
                    crate::live_hover_binding::Binding::new_native(&self.root, owner)
                        .map_err(|e| Error::NativePopup(e.to_string()))?,
                );
            }
            // Native hooks 也包含系统 Leave 过滤；仅安装而不发布绑定不能保持悬浮。
            // activate 自行复核真实鼠标所在 B，迟到样本不能夺取其他镜像的槽。
            self.hover_binding
                .as_ref()
                .unwrap()
                .activate()
                .map_err(|e| Error::NativePopup(e.to_string()))?;
            self.modal.set_owner(control);
        }
        Ok(())
    }
    pub(super) fn send_native_motion(
        &mut self,
        target: &Destination,
        packet: Message,
        event: Event,
        timeout: u32,
    ) -> Result<(), Error> {
        let native = self.local
            && !crate::live_browser_scroll::eligible(&self.root)
            && self.hover_owner.is_some()
            && target.thread()
                == unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                        self.root.hwnd(),
                        None,
                    )
                }
            && matches!(event, Event::Move { .. } | Event::Hover { .. })
            && self.modal.escape_target().is_none();
        if native {
            self.prepare_native()?;
            message::motion_native(target, packet, timeout, &mut self.modal)
        } else {
            message::send_to_with_timeout(target, packet, timeout)
        }
        .map_err(Error::Message)
    }
}
#[cfg(test)]
#[path = "live_input_control_epoch_tests.rs"]
mod epoch_tests;
#[cfg(test)]
#[path = "live_input_native_hover_tests.rs"]
mod hover_tests;
