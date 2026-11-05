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
    fn point(self) -> Option<[f64; 2]> {
        match self {
            Self::Move { point } | Self::Button { point, .. } | Self::Wheel { point, .. } => {
                Some(point)
            }
            _ => None,
        }
    }
    pub(crate) fn valid(self) -> bool {
        if self.point().is_some_and(|point| {
            point
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        }) {
            return false;
        }
        match self {
            Self::Key { code, .. } => (1..=254).contains(&code),
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
/// 仅由输入 worker 使用。实现必须对 root 和 child 重新校验，不允许裸 HWND 复用。
pub trait Backend {
    type Target: Clone;
    type Error;
    fn preflight(&mut self) -> Result<(), Self::Error>;
    /// preferred 是已有手势/按键目标；失效时只允许经验证的 root 回退。
    fn resolve(
        &mut self,
        event: Event,
        preferred: Option<Self::Target>,
    ) -> Result<Self::Target, Self::Error>;
    /// timeout_ms 是发送上限。成功必须以 API 完成标志为准，不能使用消息返回值。
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
        let result = self.dispatch_inner(sequence, event);
        // 与主干 dispatcher 一致：任何输入错误撤销控制，绝不重放失败边沿。
        if result.is_err() {
            self.cancel();
        }
        result
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
        let preferred = match event {
            Event::Key { .. } => key_slot
                .and_then(|slot| self.keys[slot].as_ref().map(|(_, target)| target.clone()))
                .or_else(|| self.keyboard.clone()),
            _ if self.buttons != 0 => self.mouse.clone(),
            _ => None,
        };
        let target = self
            .backend
            .resolve(event, preferred)
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
        self.backend
            .send(
                target,
                event,
                next_buttons,
                if matches!(event, Event::Move { .. }) {
                    50
                } else {
                    250
                },
            )
            .map_err(Error::Backend)?;
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
    /// 焦点/捕获丢失、撤销、关闭都走此入口；最多 35 次清理发送，不保留无界重试队列。
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
impl<B: Backend> Drop for Controller<B> {
    fn drop(&mut self) {
        self.cancel();
    }
}
#[cfg(test)]
#[path = "live_input_tests.rs"]
mod tests;
