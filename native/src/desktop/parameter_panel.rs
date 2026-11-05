//! 文档专属的惰性 modeless 参数窗。稳定 Rc 状态由 DocumentWindow 先于宿主析构。
use crate::desktop::{
    self, capability_parameter_menu::Snapshot, configuration_stamp::Stamp,
    parameter_action::Action, State,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "parameter_panel_controls.rs"]
mod controls;
#[path = "parameter_panel_events.rs"]
mod events;
#[path = "parameter_panel_keyboard.rs"]
mod keyboard;
#[path = "parameter_panel_layout.rs"]
mod layout;
#[path = "parameter_panel_model.rs"]
mod model;
#[cfg(test)]
#[path = "parameter_panel_tests.rs"]
mod tests;
#[path = "parameter_panel_theme.rs"]
pub(in crate::desktop) mod theme;
#[path = "parameter_panel_transaction.rs"]
mod transaction;
#[path = "parameter_panel_window.rs"]
mod window;
#[derive(Default)]
pub(in crate::desktop) struct Control {
    hwnd: Cell<HWND>,
    identity: Cell<isize>,
    pub toggle: Cell<bool>,
    pub close: Cell<bool>,
}
impl Control {
    pub(in crate::desktop) fn window(&self) -> HWND {
        let hwnd = self.hwnd.get();
        unsafe {
            if self.identity.get() != 0
                && IsWindow(Some(hwnd)).as_bool()
                && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == self.identity.get()
            {
                return hwnd;
            }
        }
        HWND::default()
    }
}
#[derive(Clone)]
pub(super) enum Intent {
    Choice(Rc<Snapshot>, usize, Option<String>),
    Texts(Rc<Snapshot>, Vec<(usize, String)>),
    Setting(Stamp, usize),
    Run(Stamp),
    Stop(Stamp),
    Relay(Stamp),
    Ports(Stamp),
    Sticker(Stamp, desktop::sticker_input::Action),
    Candidate(Rc<controls::candidates::Choice>),
    Page(usize),
    Reload,
}
struct Ui {
    host: HWND,
    host_identity: isize,
    field: Option<String>,
    parent: HWND,
    parent_identity: isize,
    committed: Cell<bool>,
    hwnd: Cell<HWND>,
    model: RefCell<Rc<model::Model>>,
    bindings: RefCell<Vec<controls::Binding>>,
    placements: RefCell<Vec<(HWND, RECT)>>,
    pending: RefCell<VecDeque<Intent>>,
    closed: Cell<bool>,
    replacing: Cell<bool>,
    page: Cell<usize>,
    scroll: Cell<i32>,
    scroll_x: Cell<i32>,
    content_height: Cell<i32>,
    status: Cell<HWND>,
    message: RefCell<String>,
    capture_windows: Rc<RefCell<Vec<HWND>>>,
}
impl Ui {
    unsafe fn state(&self) -> Option<&State> {
        if !IsWindow(Some(self.host)).as_bool()
            || GetWindowLongPtrW(self.host, GWLP_USERDATA) != self.host_identity
        {
            return None;
        }
        let state = &*(self.host_identity as *const State);
        state.lifetime.owns(self.host).then_some(state)
    }
    unsafe fn available(&self) -> bool {
        !self.closed.get() && self.state().is_some_and(available)
    }
    unsafe fn error(&self, message: &str) {
        *self.message.borrow_mut() = message.into();
        let status = self.status.get();
        if IsWindow(Some(status)).as_bool() {
            let _ = SetWindowTextW(status, &HSTRING::from(message));
        }
    }
    unsafe fn queue(&self, intent: Intent) {
        if self.replacing.get() || !self.available() {
            return;
        }
        let drafts = if matches!(&intent, Intent::Reload | Intent::Stop(_)) {
            Vec::new()
        } else {
            match controls::drafts(self) {
                Ok(drafts) => drafts,
                Err(()) => {
                    self.error("请先完成有效输入");
                    return;
                }
            }
        };
        let mut pending = self.pending.borrow().clone();
        // 候选来自既定输入/参数，不能把同次尚未提交草稿重绑定成新的选择资格。
        if matches!(&intent, Intent::Candidate(_)) && (!drafts.is_empty() || !pending.is_empty()) {
            self.error("请先提交参数更改并重新执行");
            return;
        }
        for item in drafts.into_iter().chain([intent]) {
            transaction::enqueue(&mut pending, item);
        }
        if pending.len() > 4 {
            self.error("操作队列已满，请重试");
            return;
        }
        if PostMessageW(Some(self.host), WM_NULL, WPARAM(0), LPARAM(0)).is_ok() {
            *self.pending.borrow_mut() = pending;
        } else {
            self.error("操作未入队");
        }
    }
    unsafe fn close(&self) {
        if self.state().is_none_or(|state| state.session_end.paused()) {
            return;
        }
        self.closed.set(true);
        self.pending.borrow_mut().clear();
        let focus = GetFocus();
        let restore = focus == self.hwnd.get() || IsChild(self.hwnd.get(), focus).as_bool();
        let _ = ShowWindow(self.hwnd.get(), SW_HIDE);
        let target = if self.field.is_some() {
            self.parent
        } else {
            self.host
        };
        if restore
            && self.state().is_some()
            && (self.field.is_none()
                || GetWindowLongPtrW(target, GWLP_USERDATA) == self.parent_identity)
        {
            let _ = SetFocus(Some(target));
        }
    }
}
pub(in crate::desktop) fn available(state: &State) -> bool {
    !state.session_end.paused()
        && !state.group_hidden.get()
        && !state.group_locked.get()
        && desktop::capture_input::exclusive_available(state)
}
pub(in crate::desktop) unsafe fn composing_focus() -> bool {
    keyboard::composing(GetFocus())
}
pub(in crate::desktop) unsafe fn dispatch(
    host: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !keyboard::host_key(msg, wp.0) {
        return None;
    }
    if lp.0 & (1 << 30) == 0 && state.lifetime.owns(host) && available(state) {
        state.parameters_ui.toggle.set(true);
        let _ = PostMessageW(Some(host), WM_NULL, WPARAM(0), LPARAM(0));
    }
    Some(LRESULT(0))
}
pub(in crate::desktop) struct Panel {
    ui: Rc<Ui>,
}
impl Panel {
    pub(in crate::desktop) unsafe fn new(host: HWND, state: &State) -> Result<Self> {
        Self::create(host, state, None)
    }
    pub(in crate::desktop) fn handle(&self) -> HWND {
        self.ui.hwnd.get()
    }
    pub(in crate::desktop) unsafe fn show(&self) {
        if !self.ui.available() {
            return;
        }
        let focus = GetFocus();
        let enter = focus == self.ui.host;
        let _ = ShowWindow(self.handle(), SW_SHOWNOACTIVATE);
        if enter && self.ui.available() {
            keyboard::enter(&self.ui);
        }
    }
    pub(in crate::desktop) unsafe fn translate(&self, msg: &MSG) -> bool {
        keyboard::translate(&self.ui, msg)
    }
    pub(in crate::desktop) unsafe fn chrome_key(&self, msg: &MSG) -> bool {
        let panel = self.handle();
        self.ui.available()
            && !keyboard::composing(GetFocus())
            && (msg.hwnd == panel || IsChild(panel, msg.hwnd).as_bool())
    }
    pub(in crate::desktop) unsafe fn retired(&self) -> bool {
        self.ui.closed.get()
            || self.ui.state().is_none()
            || !IsWindow(Some(self.handle())).as_bool()
    }
    pub(in crate::desktop) unsafe fn exit_idle(&self) -> bool {
        let empty = self.ui.pending.borrow().is_empty();
        empty && self.drafts_idle()
    }
    unsafe fn drafts_idle(&self) -> bool {
        let bindings = self.ui.bindings.borrow().clone();
        bindings.iter().all(|binding| match &binding.kind {
            controls::Kind::Text { initial, .. } => {
                !keyboard::composing(binding.hwnd)
                    && controls::text(binding.hwnd).is_some_and(|text| text == *initial)
            }
            _ => true,
        })
    }
    pub(in crate::desktop) unsafe fn update(&self, state: &State) -> Result<()> {
        if self.retired() {
            return Ok(());
        }
        transaction::drain(self, state);
        if self.retired() {
            return Ok(());
        }
        let current = self.ui.model.borrow().current(state);
        let waiting = state
            .parameter_view
            .borrow()
            .as_ref()
            .is_some_and(|view| !view.current(state));
        if !current && waiting {
            self.ui.error("参数同步中");
        } else if !current {
            if self.drafts_idle() {
                *self.ui.model.borrow_mut() = Rc::new(self.ui.capture(state));
                controls::rebuild(&self.ui)?;
            } else {
                self.ui.error("配置已变化，草稿未提交");
            }
        }
        controls::availability(&self.ui);
        Ok(())
    }
    unsafe fn apply(&self, state: &State, intent: Intent) -> Result<()> {
        match intent {
            Intent::Choice(snapshot, index, text) => {
                if !snapshot.current(state) {
                    self.ui.error("配置已变化，请重试");
                    return Ok(());
                }
                let before = Stamp::capture(state);
                if snapshot.submit_guarded(self.ui.host, state, index, text.as_deref(), &|| {
                    self.ui.available()
                })? && before != Stamp::capture(state)
                {
                    self.ui.committed.set(true);
                }
                if let Some(text) = &text {
                    for binding in self.ui.bindings.borrow_mut().iter_mut() {
                        if let controls::Kind::Text {
                            snapshot: bound,
                            index: bi,
                            initial,
                            ..
                        } = &mut binding.kind
                        {
                            if Rc::ptr_eq(bound, &snapshot) && *bi == index {
                                *initial = text.clone();
                            }
                        }
                    }
                }
                self.ui.error("");
            }
            Intent::Texts(..) => unreachable!("text batches are drained transactionally"),
            Intent::Candidate(choice) => {
                if !choice.submit(self.ui.host, state, &|| {
                    self.ui.available() && self.drafts_idle()
                })? {
                    self.ui.error("候选或参数已变化，请重新执行");
                } else {
                    self.ui.committed.set(true);
                    self.ui.error("");
                }
            }
            Intent::Page(page) => {
                self.ui.page.set(page);
                self.ui.scroll.set(0);
                controls::rebuild(&self.ui)?;
            }
            Intent::Reload => {
                let bindings = self.ui.bindings.borrow().clone();
                if bindings.iter().any(|b| keyboard::composing(b.hwnd)) {
                    return Ok(());
                }
                if state
                    .parameter_view
                    .borrow()
                    .as_ref()
                    .is_some_and(|view| !view.current(state))
                {
                    self.ui.error("参数同步中");
                    return Ok(());
                }
                *self.ui.model.borrow_mut() = Rc::new(self.ui.capture(state));
                self.ui.error("");
                controls::rebuild(&self.ui)?;
            }
            Intent::Setting(stamp, index) if stamp.matches(state) => {
                desktop::execution_settings::toggle(
                    self.ui.host,
                    state,
                    desktop::execution_settings::FIRST + index,
                )?
            }
            Intent::Run(stamp) if stamp.matches(state) => {
                desktop::execution_start::request_after_parameter(self.ui.host, state)?
            }
            Intent::Stop(stamp) if stamp.matches(state) => {
                desktop::execution_cancel::stop(self.ui.host, state)?;
            }
            Intent::Relay(stamp) if stamp.matches(state) => {
                desktop::sticker_propagation::toggle(self.ui.host, state)?
            }
            Intent::Ports(stamp) if stamp.matches(state) => {
                if !desktop::application::chrome::ports::request(&state.chrome, self.ui.host) {
                    self.ui.error("端口设置暂不可用");
                }
            }
            Intent::Sticker(stamp, action) => {
                let before = Stamp::capture(state);
                let outcome =
                    desktop::sticker_input::apply(self.ui.host, state, &stamp, action, &|| {
                        self.ui.available()
                    })?;
                self.sticker_outcome(state, &before, outcome);
            }
            _ => self.ui.error("配置已变化，请重试"),
        }
        Ok(())
    }
    unsafe fn sticker_outcome(
        &self,
        state: &State,
        before: &Stamp,
        outcome: desktop::parameter_file_input::Outcome,
    ) {
        use desktop::parameter_file_input::Outcome;
        match outcome {
            Outcome::Rejected => self.ui.error("图片输入配置已变化或暂不可用"),
            Outcome::Cancelled => self.ui.error(""),
            Outcome::Accepted => {
                self.ui.error("");
                if before != &Stamp::capture(state) {
                    self.ui.committed.set(true);
                }
            }
        }
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        unsafe {
            let hwnd = self.handle();
            if let Some(state) = self.ui.state() {
                if state.parameters_ui.hwnd.get() == hwnd {
                    state.parameters_ui.hwnd.set(HWND::default());
                    state.parameters_ui.identity.set(0);
                }
            }
            if IsWindow(Some(hwnd)).as_bool()
                && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == Rc::as_ptr(&self.ui) as isize
            {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
