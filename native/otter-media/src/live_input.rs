//! 有界 Live 输入控制状态；transport 负责身份/权限、坐标转换和超时，不隐式授权。
pub const MAX_SEQUENCE: u64 = 9_007_199_254_740_991;
const MAX_KEYS: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}
impl Button {
    fn mask(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 2,
            Self::Middle => 4,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// 本地窗口消息；远程合同不因此自动开放文字或 hover 能力。
    Character {
        unit: u16,
    },
    Hover {
        point: [f64; 2],
    },
    Leave,
    Move {
        point: [f64; 2],
    },
    Button {
        point: [f64; 2],
        button: Button,
        down: bool,
        double: bool,
    },
    Wheel {
        point: [f64; 2],
        delta: i32,
        horizontal: bool,
    },
    Key {
        code: u16,
        down: bool,
    },
}
impl Event {
    pub fn message_timeout_ms(self) -> u32 {
        if matches!(self, Self::Move { .. } | Self::Hover { .. } | Self::Leave) {
            50
        } else {
            250
        }
    }
    fn point(self) -> Option<[f64; 2]> {
        match self {
            Self::Move { point }
            | Self::Hover { point }
            | Self::Button { point, .. }
            | Self::Wheel { point, .. } => Some(point),
            _ => None,
        }
    }
    pub fn valid(self) -> bool {
        if self.point().is_some_and(|point| {
            point
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        }) {
            return false;
        }
        match self {
            Self::Key { code, .. } => (1..=254).contains(&code),
            Self::Character { unit } => unit != 0,
            Self::Wheel { delta, .. } => delta != 0 && delta.unsigned_abs() <= 1200,
            Self::Button {
                down: false,
                double: true,
                ..
            } => false,
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error<E> {
    Disabled,
    Sequence,
    InvalidEvent,
    KeyLimit,
    Backend(E),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    pub released: u8,
    pub failed: u8,
    pub skipped: u8,
}
/// Completed 是窗口过程已经返回；Pending 仅表示有限槽位中的本地模态输入已提交。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Completed,
    Pending,
    IgnoredWhileModal,
}
/// 仅由输入 worker 使用。实现必须对 root 和 child 重新校验，不允许裸 HWND 复用。
pub trait Backend {
    type Target: Clone;
    type Error;
    /// 观察异步完成；失败由 Controller 撤销并归还原 held 状态，不等待下一条输入。
    fn idle(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn delivery(&mut self) -> Delivery {
        Delivery::Completed
    }
    fn pending_delivery(&mut self) -> bool {
        false
    }
    /// 已有模态释放未返回时，可明确消费重复手势；不能宣称消息已投递。
    fn skip_modal_input(&mut self, _event: Event) -> bool {
        false
    }
    /// 复合后端可声明独立的有限发送预算；普通消息与远程路径保持原预算。
    fn send_timeout_ms(&self, event: Event) -> u32 {
        event.message_timeout_ms()
    }
    fn preflight(&mut self) -> Result<(), Self::Error>;
    /// 清理最后的悬浮目标，不重新命中，不移动系统指针。
    fn leave(&mut self) -> Cleanup {
        Cleanup::default()
    }
    /// preferred 是已有手势/按键目标；失效时只允许经验证的 root 回退。
    fn resolve(
        &mut self,
        event: Event,
        preferred: Option<Self::Target>,
    ) -> Result<Self::Target, Self::Error>;
    /// 新按键/文字可跟随源焦点；fallback 只是上次键盘目标，不是欠释放的 held 目标。
    fn resolve_keyboard(
        &mut self,
        event: Event,
        fallback: Option<Self::Target>,
    ) -> Result<Self::Target, Self::Error> {
        self.resolve(event, fallback)
    }
    /// timeout_ms 是发送上限。默认以 API 完成标志为准；模态准入必须通过 delivery
    /// 明确标记 Pending，不能用消息返回值推断完成，也不能把准入伪装为完成。
    fn send(
        &mut self,
        target: Self::Target,
        event: Event,
        buttons: u8,
        timeout_ms: u32,
    ) -> Result<(), Self::Error>;
    /// 清理不得触发点击补偿。源身份/权限失效返回 None，绝不向复用句柄发送。
    fn cleanup_target(&mut self, target: Self::Target) -> Option<Self::Target>;
    /// 与普通 send 分开：只发送释放，清除延迟点击动作，使用 50 ms 上限。
    fn release(
        &mut self,
        target: Self::Target,
        event: Event,
        buttons: u8,
    ) -> Result<(), Self::Error>;
}
pub struct Controller<B: Backend> {
    backend: B,
    enabled: bool,
    sequence: u64,
    mouse: Option<B::Target>,
    keyboard: Option<B::Target>,
    buttons: u8,
    point: [f64; 2],
    keys: [Option<(u16, B::Target)>; MAX_KEYS],
    cleanup: Cleanup,
}
impl<B: Backend> Controller<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            enabled: false,
            sequence: 0,
            mouse: None,
            keyboard: None,
            buttons: 0,
            point: [0.0; 2],
            keys: std::array::from_fn(|_| None),
            cleanup: Cleanup::default(),
        }
    }
    /// 必须由显式控制请求调用，不能在捕获启动或普通绘制时自动启用。
    pub fn enable(&mut self) -> Result<(), Error<B::Error>> {
        if let Err(error) = self.backend.preflight() {
            self.cancel();
            return Err(Error::Backend(error));
        }
        self.enabled = true;
        Ok(())
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn last_sequence(&self) -> u64 {
        self.sequence
    }
    pub fn last_cleanup(&self) -> Cleanup {
        self.cleanup
    }
    pub fn dispatch(&mut self, sequence: u64, event: Event) -> Result<(), Error<B::Error>> {
        self.dispatch_status(sequence, event).map(|_| ())
    }
    pub fn idle(&mut self) -> Result<(), Error<B::Error>> {
        self.backend.idle().map_err(|error| {
            self.cancel();
            Error::Backend(error)
        })
    }
    pub fn pending_delivery(&mut self) -> bool {
        self.backend.pending_delivery()
    }
    pub fn dispatch_status(
        &mut self,
        sequence: u64,
        event: Event,
    ) -> Result<Delivery, Error<B::Error>> {
        let result = self.dispatch_inner(sequence, event);
        // 与主干 dispatcher 一致：任何输入错误撤销控制，绝不重放失败边沿。
        if result.is_err() {
            self.cancel();
        }
        result.map(|_| self.backend.delivery())
    }
    fn dispatch_inner(&mut self, sequence: u64, event: Event) -> Result<(), Error<B::Error>> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if sequence == 0 || sequence > MAX_SEQUENCE || sequence <= self.sequence {
            return Err(Error::Sequence);
        }
        self.backend.preflight().map_err(Error::Backend)?;
        // 校验源之后、转换/发送之前消耗序号，失败/超时也不能重发同一边沿。
        self.sequence = sequence;
        if !event.valid() {
            return Err(Error::InvalidEvent);
        }
        if self.backend.skip_modal_input(event) {
            return Ok(());
        }
        let key_slot = match event {
            Event::Key { code, down } => {
                let existing = self
                    .keys
                    .iter()
                    .position(|held| held.as_ref().is_some_and(|(key, _)| *key == code));
                if down {
                    Some(
                        existing
                            .or_else(|| self.keys.iter().position(Option::is_none))
                            .ok_or(Error::KeyLimit)?,
                    )
                } else {
                    existing
                }
            }
            _ => None,
        };
        // 先登记可能按下，保证投递失败仍会释放；明确未投递时恢复原 held/键盘目标。
        // 不能直接清空 slot：重复 down 之前可能已有真实按下尚欠 up。
        let key_before =
            key_slot.map(|slot| (slot, self.keys[slot].clone(), self.keyboard.clone()));
        let preferred = match event {
            Event::Character { .. } => self.keyboard.clone(),
            Event::Key { .. } => key_slot
                .and_then(|slot| self.keys[slot].as_ref().map(|(_, target)| target.clone()))
                .or_else(|| self.keyboard.clone()),
            _ if self.buttons != 0 => self.mouse.clone(),
            _ => None,
        };
        let fresh_keyboard = matches!(event, Event::Character { .. })
            || matches!(event, Event::Key { .. })
                && key_slot.is_none_or(|slot| self.keys[slot].is_none());
        let target = if fresh_keyboard {
            self.backend.resolve_keyboard(event, preferred)
        } else {
            self.backend.resolve(event, preferred)
        }
        .map_err(Error::Backend)?;
        if let Some(point) = event.point() {
            self.point = point;
        }
        let mut next_buttons = self.buttons;
        match event {
            Event::Button { button, down, .. } => {
                if down {
                    next_buttons |= button.mask();
                    self.mouse = Some(target.clone());
                    self.keyboard = Some(target.clone());
                    // 先记录可能按下；发送超时也需要尝试释放。
                    self.buttons = next_buttons;
                } else {
                    next_buttons &= !button.mask();
                }
            }
            Event::Key { code, down: true } => {
                self.keys[key_slot.unwrap()] = Some((code, target.clone()));
                self.keyboard = Some(target.clone());
            }
            _ => {}
        }
        let timeout_ms = self.backend.send_timeout_ms(event);
        self.backend
            .send(target, event, next_buttons, timeout_ms)
            .map_err(Error::Backend)?;
        if matches!(event, Event::Key { .. })
            && self.backend.delivery() == Delivery::IgnoredWhileModal
        {
            if let Some((slot, held, keyboard)) = key_before {
                self.keys[slot] = held;
                self.keyboard = keyboard;
            }
            return Ok(());
        }
        match event {
            Event::Button { .. } => {
                self.buttons = next_buttons;
                if self.buttons == 0 {
                    self.mouse = None;
                }
            }
            Event::Key { down: false, .. } => {
                if let Some(slot) = key_slot {
                    self.keys[slot] = None;
                }
            }
            _ => {}
        }
        Ok(())
    }
    /// 焦点/捕获丢失、撤销、关闭都走此入口；最多 35 次按键/按钮释放及一次悬浮退出，不保留无界重试队列。
    /// 失败/跳过数明确保留，不能把清空本地 held 状态宣称为 OS 已确认释放。
    pub fn cancel(&mut self) -> Cleanup {
        self.enabled = false;
        let mut report = Cleanup::default();
        for button in [Button::Left, Button::Right, Button::Middle] {
            if self.buttons & button.mask() == 0 {
                continue;
            }
            self.buttons &= !button.mask();
            if let Some(target) = self.mouse.clone() {
                self.release(
                    target,
                    Event::Button {
                        point: self.point,
                        button,
                        down: false,
                        double: false,
                    },
                    &mut report,
                );
            } else {
                report.skipped += 1;
            }
        }
        for slot in 0..MAX_KEYS {
            if let Some((code, target)) = self.keys[slot].take() {
                self.release(target, Event::Key { code, down: false }, &mut report);
            }
        }
        let hover = self.backend.leave();
        report.released += hover.released;
        report.failed += hover.failed;
        report.skipped += hover.skipped;
        self.mouse = None;
        self.keyboard = None;
        self.cleanup = report;
        report
    }
    fn release(&mut self, target: B::Target, event: Event, report: &mut Cleanup) {
        let Some(target) = self.backend.cleanup_target(target) else {
            report.skipped += 1;
            return;
        };
        if self.backend.release(target, event, self.buttons).is_ok() {
            report.released += 1;
        } else {
            report.failed += 1;
        }
    }
}
#[cfg(test)]
#[path = "live_input_keyboard_focus_tests.rs"]
mod keyboard_focus_tests;
impl<B: Backend> Drop for Controller<B> {
    fn drop(&mut self) {
        self.cancel();
    }
}
#[cfg(test)]
#[path = "live_input_ignored_key_tests.rs"]
mod ignored_key_tests;
#[cfg(test)]
#[path = "live_input_tests.rs"]
mod tests;
