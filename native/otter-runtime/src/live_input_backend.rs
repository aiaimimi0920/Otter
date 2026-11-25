//! Controller 的 Win32 后端：路由、坐标、受限消息组合；必须由独立输入 worker 拥有。
use super::{
    input_coordinates::{self as coordinates, FrameRegion, Mapped},
    input_message::{self as message, Message},
    input_routing as routing, input_security,
    input_target::{self, Destination},
    Target,
};
use crate::live_input::{self, Event};
#[path = "live_input_browser_scroll.rs"]
mod browser_scroll;
#[path = "live_input_button.rs"]
pub mod button;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Security(input_security::Denied),
    Target(input_target::Error),
    Coordinates(coordinates::Error),
    Routing(routing::Error),
    Message(message::Error),
    InvalidDelivery,
    BrowserScroll(String),
    BrowserTransition(String),
    BrowserHover(String),
    NativePopup(String),
    PopupScope(String),
    ControlRevoked,
}
struct MousePosition {
    target: Destination,
    mapped: Mapped,
}
pub struct Backend {
    root: Target,
    root_destination: Destination,
    region: Option<FrameRegion>,
    mouse: Option<MousePosition>,
    // 与释放坐标分开：仅 Win32 指针消息尝试投递后才承担 Leave 清理。
    hovered: Option<Destination>,
    button: button::Tracker,
    local: bool,
    popup_scope: Option<super::popup::Scope>,
    modal: message::Modal,
    last_delivery: live_input::Delivery,
    suppressed_right_down: bool,
    browser_scroll: Option<crate::live_browser_scroll::Worker>,
    #[cfg(any(test, feature = "test-support"))]
    scroll_in_test_harness: bool,
    hover_owner: Option<crate::live_hover_owner::Control>,
    hover_binding: Option<crate::live_hover_binding::Binding>,
}
impl Backend {
    // 普通桌面应用启动播放等操作会短时阻塞 UI；无按键边沿的消息也使用现有
    // 普通输入预算，而非更短的浏览器 hover 预算。不重发或改变同步消息顺序。
    fn native_motion_timeout(&self) -> u32 {
        if self.local && !crate::live_browser_scroll::eligible(&self.root) {
            250
        } else {
            50
        }
    }
    /// 创建不启用 Controller；region 必须是用户确认的窗口内物理裁剪，不能用于桌面区域。
    pub fn new(root: Target, region: Option<FrameRegion>) -> Result<Self, Error> {
        let root_destination = Destination::bind(&root, root.hwnd()).map_err(Error::Target)?;
        Ok(Self {
            root,
            root_destination,
            region,
            mouse: None,
            hovered: None,
            button: button::Tracker::default(),
            local: false,
            popup_scope: None,
            modal: message::Modal::default(),
            last_delivery: live_input::Delivery::Completed,
            suppressed_right_down: false,
            browser_scroll: None,
            #[cfg(any(test, feature = "test-support"))]
            scroll_in_test_harness: false,
            hover_owner: None,
            hover_binding: None,
        })
    }
    /// 仅本机 Live 授权入口；远程控制维持原 Win32 权限与行为。
    pub fn new_local(root: Target, region: Option<FrameRegion>) -> Result<Self, Error> {
        let mut backend = Self::new(root, region)?;
        backend.local = true;
        Ok(backend)
    }
    fn exit_browser_hover(&self) -> Result<(), Error> {
        // Chromium 的 Leave 重新读取真实光标位置；鼠标在被遮挡的 A 范围内时，
        // 单发 Leave 仍可残留 DOM hover。先给 root 明确的虚拟移出，不改系统光标。
        if self.local && crate::live_browser_scroll::eligible(&self.root) {
            message::release_to(&self.root_destination, Message::exit_motion())
                .map_err(Error::Message)?;
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        target: &Destination,
        event: Event,
        buttons: u8,
    ) -> Result<Message, Error> {
        if let Event::Character { unit } = event {
            return Message::character(unit).map_err(Error::Message);
        }
        if event == Event::Leave {
            return Ok(Message::leave());
        }
        if let Event::Key { code, down } = event {
            return Message::key(code, down).map_err(Error::Message);
        }
        let point = match event {
            Event::Move { point }
            | Event::Hover { point }
            | Event::Button { point, .. }
            | Event::Wheel { point, .. } => point,
            _ => unreachable!(),
        };
        let mapped = self.map_point(target, point)?;
        let packet = match event {
            Event::Hover { .. } => Message::hover(mapped.destination_client, buttons),
            Event::Move { .. } => Message::motion(mapped.destination_client, buttons),
            Event::Button {
                button,
                down,
                double,
                ..
            } => Message::button(mapped.destination_client, buttons, button, down, double),
            Event::Wheel {
                delta, horizontal, ..
            } => Message::wheel(mapped.screen, buttons, delta, horizontal),
            _ => unreachable!(),
        }
        .map_err(Error::Message)?;
        // 发送前保留最后有效位置，超时后的释放不能重新计算或重新命中。
        self.mouse = Some(MousePosition {
            target: target.clone(),
            mapped,
        });
        Ok(packet)
    }
}
impl live_input::Backend for Backend {
    type Target = Destination;
    type Error = Error;
    fn idle(&mut self) -> Result<(), Error> {
        self.modal.check_result().map_err(Error::Message)
    }
    fn delivery(&mut self) -> live_input::Delivery {
        self.last_delivery
    }
    fn pending_delivery(&mut self) -> bool {
        self.modal.pending()
    }
    fn skip_modal_input(&mut self, event: Event) -> bool {
        let Event::Button {
            button: live_input::Button::Right,
            down,
            ..
        } = event
        else {
            return self.ignore_inactive_motion(event);
        };
        // 被消费的 down/up 必须成对；即使中途菜单关闭，也不能把剩下的 up 单独送达。
        if self.modal.escape_target().is_some() || self.suppressed_right_down {
            self.suppressed_right_down = down;
            self.last_delivery = live_input::Delivery::IgnoredWhileModal;
            true
        } else {
            false
        }
    }
    fn send_timeout_ms(&self, event: Event) -> u32 {
        if matches!(event, Event::Wheel { .. })
            && self.local
            && crate::live_browser_scroll::eligible(&self.root)
        {
            crate::live_browser_scroll::TIMEOUT_MS
                + if self.browser_scroll.is_none() {
                    crate::live_browser_scroll::STARTUP_TIMEOUT_MS
                } else {
                    0
                }
        } else if matches!(
            event,
            Event::Move { .. } | Event::Hover { .. } | Event::Leave
        ) {
            self.native_motion_timeout()
        } else {
            event.message_timeout_ms()
        }
    }
    fn preflight(&mut self) -> Result<(), Error> {
        self.check_control()?;
        self.check_popup_scope()?;
        self.modal.check_result().map_err(Error::Message)?;
        input_security::preflight(&self.root).map_err(Error::Security)?;
        if unsafe { windows::Win32::UI::WindowsAndMessaging::IsIconic(self.root.hwnd()) }.as_bool()
        {
            return Err(Error::Message(message::Error::SourceTransition));
        }
        Ok(())
    }
    fn resolve(
        &mut self,
        event: Event,
        preferred: Option<Destination>,
    ) -> Result<Destination, Error> {
        if matches!(event, Event::Key { code: 0x1b, .. }) {
            if let Some(target) = self.modal.escape_target() {
                return Ok(target.clone());
            }
        }
        if matches!(
            event,
            Event::Button {
                button: live_input::Button::Right,
                down: true,
                ..
            }
        ) && self.modal.in_flight()
        {
            return Err(Error::Message(message::Error::ModalPending));
        }
        if let Some(target) = preferred {
            return routing::retained(&self.root, &target).map_err(Error::Routing);
        }
        match event {
            Event::Key { .. } | Event::Character { .. } => {
                routing::keyboard(&self.root).map_err(Error::Routing)
            }
            Event::Leave => Ok(self
                .hovered
                .as_ref()
                .unwrap_or(&self.root_destination)
                .clone()),
            Event::Move { point }
            | Event::Hover { point }
            | Event::Button { point, .. }
            | Event::Wheel { point, .. } => {
                let mapped = self.map_point(&self.root_destination, point)?;
                routing::pointer(&self.root, mapped.root_client).map_err(Error::Routing)
            }
        }
    }
    fn resolve_keyboard(
        &mut self,
        event: Event,
        fallback: Option<Destination>,
    ) -> Result<Destination, Error> {
        if self.local
            && !matches!(event, Event::Key { code: 0x1b, .. })
            && !crate::live_browser_scroll::eligible(&self.root)
        {
            routing::keyboard_with_fallback(&self.root, fallback.as_ref()).map_err(Error::Routing)
        } else {
            self.resolve(event, fallback)
        }
    }
    fn send(
        &mut self,
        target: Destination,
        event: Event,
        buttons: u8,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        self.check_control()?;
        self.check_popup_scope()?;
        self.last_delivery = live_input::Delivery::Completed;
        if event == Event::Leave && self.hovered.is_none() {
            return Ok(());
        }
        let expected = self.send_timeout_ms(event);
        if timeout_ms != expected {
            return Err(Error::InvalidDelivery);
        }
        let packet = self.prepare(&target, event, buttons)?;
        let point = self
            .mouse
            .as_ref()
            .map_or([0, 0], |mouse| mouse.mapped.destination_client);
        // 即使滚轮改走 UIA，也必须结束原按钮手势，不能保留延迟点击补偿。
        let candidate = self.button.track(&target, event, point, buttons);
        let Some(packet) = self.scroll_packet(&target, event, buttons, packet)? else {
            return Ok(());
        };
        // UIA 查询/滚动未向 HWND 投递鼠标消息，不建立新的悬浮清理责任。
        // 点击/滚轮也可能先于 move 到达新子窗；在真实投递前先退出旧目标。
        if matches!(
            event,
            Event::Move { .. } | Event::Hover { .. } | Event::Button { .. } | Event::Wheel { .. }
        ) {
            if self
                .hovered
                .as_ref()
                .is_some_and(|old| !old.same_target(&target))
            {
                let old = self.hovered.take().unwrap();
                // 导航可替换 renderer HWND；失效目标不清理，不能向复用句柄投递。
                if old.preflight().is_ok() {
                    message::release_to_with_timeout(
                        &old,
                        Message::leave(),
                        self.native_motion_timeout(),
                    )
                    .map_err(Error::Message)?;
                }
            }
            // 超时仍可能已投递，必须在调用前登记；不能只清理成功的消息。
            self.hovered = Some(target.clone());
            if let Some(control) = self
                .hover_owner
                .filter(|_| self.local && crate::live_browser_scroll::eligible(&self.root))
            {
                let owner = control.identity();
                if self.hover_binding.is_none() {
                    self.hover_binding = Some(
                        crate::live_hover_binding::Binding::new(&self.root, owner)
                            .map_err(|e| Error::BrowserHover(e.to_string()))?,
                    );
                }
                self.hover_binding
                    .as_ref()
                    .unwrap()
                    .activate()
                    .map_err(|e| Error::BrowserHover(e.to_string()))?;
            }
        }
        if event == Event::Leave && buttons == 0 {
            self.exit_browser_hover()?;
        }
        let local_modal = self.local_button_modal(event);
        let delivery = if matches!(event, Event::Key { code: 0x1b, .. })
            && self.modal.escape_target().is_some()
        {
            self.modal.post_escape(&target, packet).map(|posted| {
                self.last_delivery = if posted {
                    live_input::Delivery::Pending
                } else {
                    live_input::Delivery::IgnoredWhileModal
                };
            })
        } else if local_modal
            && (matches!(
                event,
                Event::Button {
                    button: live_input::Button::Right,
                    down: false,
                    ..
                }
            ) || (matches!(
                event,
                Event::Button {
                    button: live_input::Button::Left,
                    ..
                }
            ) && candidate.is_none()
                && self.modal.escape_target().is_none()))
        {
            // 左键按下、双击或松开都可打开模态窗；按下准入后仍由 Controller
            // 持有释放责任。只在槽空闲时接管，已有模态中的点击仍可
            // 关闭菜单，不覆盖原回调；WinForms 补偿仍要求同步确认 mouse-up。
            self.prepare_native()?;
            let result = message::button_modal(&target, packet, &mut self.modal);
            if result.is_ok() && self.modal.in_flight() {
                self.last_delivery = live_input::Delivery::Pending;
            }
            result
        } else if matches!(
            event,
            Event::Move { .. } | Event::Hover { .. } | Event::Leave
        ) {
            self.send_native_motion(&target, packet, event, timeout_ms)?;
            Ok(())
        } else {
            message::send_to(&target, packet)
        };
        if let Err(error) = delivery {
            self.button.clear();
            return Err(Error::Message(error));
        }
        // Leave 结束悬浮，不代表 held 按钮已经释放；cancel 仍需最后映射坐标。
        if event == Event::Leave {
            self.hovered = None;
            if buttons == 0 {
                self.mouse = None;
            }
        }
        if let Some(candidate) = candidate {
            button::complete(candidate, point, local_modal.then_some(&mut self.modal))?;
            if local_modal && self.modal.in_flight() {
                self.last_delivery = live_input::Delivery::Pending;
            }
        }
        Ok(())
    }
    fn leave(&mut self) -> live_input::Cleanup {
        self.suppressed_right_down = false;
        let mut report = live_input::Cleanup::default();
        let may_signal = self.popup_cleanup_allowed();
        if self.modal.cancel(may_signal) {
            report.failed += 1;
        }
        if let Some(binding) = self.hover_binding.take() {
            if let Err(error) = binding.close() {
                eprintln!("native-live-hover-cleanup: {error}");
                report.failed += 1;
            }
        }
        self.mouse = None; // Controller 已完成 held-button 释放。
        if let Some(target) = self.hovered.take() {
            // 悬浮退出不计入按住按键的释放数；目标已销毁时无需清理悬浮。
            if self.popup_cleanup_allowed() && target.preflight().is_ok() {
                if self.exit_browser_hover().is_err() {
                    report.failed += 1;
                }
                if message::release_to_with_timeout(
                    &target,
                    Message::leave(),
                    self.native_motion_timeout(),
                )
                .is_err()
                {
                    report.failed += 1;
                }
            }
        }
        report
    }
    fn cleanup_target(&mut self, target: Destination) -> Option<Destination> {
        self.button.clear();
        self.popup_cleanup_allowed()
            .then(|| routing::retained(&self.root, &target).ok())
            .flatten()
    }
    fn release(&mut self, target: Destination, event: Event, buttons: u8) -> Result<(), Error> {
        self.button.clear();
        if !self.popup_cleanup_allowed() {
            return Err(Error::PopupScope(
                "Live popup instance changed before release".into(),
            ));
        }
        let packet = match event {
            Event::Key { code, down: false } => Message::key(code, false),
            Event::Button {
                button,
                down: false,
                ..
            } => {
                let mouse = self.mouse.as_ref().ok_or(Error::InvalidDelivery)?;
                let point = if mouse.target.same_target(&target) {
                    mouse.mapped.destination_client
                } else if target.same_target(&self.root_destination) {
                    mouse.mapped.root_client
                } else {
                    return Err(Error::InvalidDelivery);
                };
                Message::button(point, buttons, button, false, false)
            }
            _ => return Err(Error::InvalidDelivery),
        }
        .map_err(Error::Message)?;
        if matches!(
            event,
            Event::Key {
                code: 0x1b,
                down: false
            }
        ) && self.modal.escape_needs_release(&target)
        {
            // 不能用同步 up 越过已入队的 down。取消不等待源消息泵，保持结果未知。
            self.modal
                .post_escape(&target, packet)
                .map_err(Error::Message)?;
            return Err(Error::Message(message::Error::DeliveryFailed));
        }
        // 独立 cleanup 路径：不再次命中/变换、不进行任何按钮点击补偿。
        message::release_to(&target, packet).map_err(Error::Message)
    }
}
#[path = "live_input_native_backend.rs"]
pub mod native_backend;
#[path = "live_input_popup_backend.rs"]
pub mod popup_backend;
#[cfg(test)]
#[path = "live_input_backend_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "live_input_winforms_tests.rs"]
mod winforms_tests;
