//! 窗口输入控制状态：队列准入、暂停/撤销及同步悬浮资格；不持有源线程钩子。
use super::*;
impl Control {
    pub(in super::super) fn source_input_allowed(&self) -> bool {
        self.allowed
    }
    pub(in super::super) fn wheel_wanted(&self) -> bool {
        self.allowed && self.desired && !self.remote && !self.failed
    }
    pub(in super::super) fn cancel(&mut self) {
        self.desired = false;
        self.shown = None;
        self.pause();
    }
    pub(super) fn pause(&mut self) {
        self.request = Some(false);
        self.clear();
    }
    pub(super) fn clear(&mut self) {
        self.native_active(false);
        self.hover_active(false);
        self.tracking = pointer_tracking::Tracking::default();
        self.ime.clear();
        self.enabled = false;
        self.pending = false;
        self.buttons = 0;
        self.pointer = None;
        self.held_keys = [0; 4];
        self.events.clear();
        if let Some(stop) = self.stop.take() {
            stop.request();
        }
    }
    pub(super) fn push(&mut self, event: Event) -> bool {
        if let Event::Key { code, down: false } = event {
            if self.held_keys[usize::from(code) / 64] & (1u64 << (code % 64)) == 0 {
                // 交接已排入原目标 up，或当前 B 从未准入 down；迟到物理 up 不能
                // 再按新焦点路由。已排入 FIFO 的释放与 worker 的取消责任不受影响。
                return true;
            }
        }
        if !queue::push_native(&mut self.events, event, self.buttons) {
            eprintln!("native-live-input-pause: reason=queue-capacity");
            // 过载只清积压，保留用户控制意图；不得重放旧输入。
            self.pause();
            return false;
        }
        if let Event::Key { code, down } = event {
            // 本地 decoder 已将 VK 限制在 1..=254；这里只记录 UI 准入状态，
            // 真正的目标绑定和释放责任仍属于 worker 的 Controller。
            let word = &mut self.held_keys[usize::from(code) / 64];
            let mask = 1u64 << (code % 64);
            if down {
                *word |= mask;
            } else {
                *word &= !mask;
            }
        }
        match event {
            Event::Leave => self.hover_active(false),
            Event::Move { point }
            | Event::Hover { point }
            | Event::Button { point, .. }
            | Event::Wheel { point, .. } => {
                self.pointer = Some(point);
                self.hover_active(true);
            }
            _ => {}
        }
        true
    }
    pub(super) fn button_edge(&mut self, mask: u8, down: bool) -> bool {
        if down {
            self.buttons |= mask;
        } else {
            if self.buttons & mask == 0 {
                return false; // 已在焦点交接时排入松开，迟到物理 up 不能再次投递。
            }
            self.buttons &= !mask;
        }
        true
    }
    pub(super) fn handoff_source(&mut self) -> bool {
        if self.buttons != 0 {
            let Some(point) = self.pointer else {
                return false; // 没有已准入坐标不能发明松开，由原 Controller 取消清理。
            };
            for (button, mask) in [
                (crate::live_input::Button::Left, 1),
                (crate::live_input::Button::Right, 2),
                (crate::live_input::Button::Middle, 4),
            ] {
                if self.buttons & mask != 0 {
                    if !self.push(Event::Button {
                        point,
                        button,
                        down: false,
                        double: false,
                    }) {
                        return false;
                    }
                    self.buttons &= !mask;
                }
            }
        }
        self.handoff_keys()
    }
    pub(super) fn handoff_keys(&mut self) -> bool {
        if self.buttons != 0 {
            return false; // 捕获/拖动丢失仍走原暂停，不能带着按下进入另一镜像。
        }
        // 按原 FIFO 排在已准入 down 后。目标与释放责任由原 worker 的 held slot 保留，
        // 不停止 worker/撤销 epoch，否则会同时取消仍打开的源模态菜单。
        let held = self.held_keys;
        for code in 1u16..=254 {
            if held[usize::from(code) / 64] & (1u64 << (code % 64)) != 0
                && !self.push(Event::Key { code, down: false })
            {
                return false; // 满载 push 已撤销，Controller 仍负责尚未投递的释放。
            }
        }
        true
    }
    pub(super) fn native_active(&self, active: bool) {
        if let Some(owner) = &self.hover {
            if let Err(error) = owner.control(active) {
                eprintln!("native-live-control-scope: {error}");
            }
        }
    }
    fn hover_active(&self, active: bool) {
        if let Some(owner) = &self.hover {
            if let Err(error) = owner.active(active) {
                eprintln!("native-live-hover-scope: {error}");
            }
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_input_key_tests.rs"]
mod key_tests;
impl LiveWindow {
    pub(crate) fn hover_identity(&self) -> Option<crate::live_hover_owner::Identity> {
        self.state
            .input
            .borrow()
            .hover
            .as_ref()
            .map(|owner| owner.identity())
    }
}
