//! A document-owned lazy modeless parameter window. DocumentWindow destroys the stable Rc state before the host.
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
#[path = "parameter_panel_auxiliary.rs"]
mod auxiliary;
#[path = "parameter_panel_controls.rs"]
mod controls;
#[path = "parameter_panel_events.rs"]
mod events;
#[path = "parameter_panel_inline.rs"]
mod inline;
#[path = "parameter_panel_inline_input.rs"]
mod inline_input;
#[path = "parameter_panel_keyboard.rs"]
mod keyboard;
#[path = "parameter_panel_language.rs"]
mod language;
#[path = "parameter_panel_layout.rs"]
mod layout;
pub(in crate::desktop) use inline::Inline;
#[path = "parameter_panel_model.rs"]
mod model;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
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
    pub expanded: Cell<bool>,
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
    pub(in crate::desktop) unsafe fn capture_window(&self) -> HWND {
        let hwnd = self.window();
        if hwnd.is_invalid() {
            hwnd
        } else {
            GetAncestor(hwnd, GA_ROOT)
        }
    }
}
#[derive(Clone)]
pub(super) enum Intent {
    Choice(Rc<Snapshot>, usize, Option<String>),
    Texts(Rc<Snapshot>, Vec<(usize, String)>),
    Setting(Stamp, desktop::node_settings::Action),
    Run(Stamp),
    Stop(Stamp),
    Ports(Stamp),
    Sticker(Stamp, desktop::sticker_input::Action),
    Candidate(Rc<controls::candidates::Choice>),
    Page(usize),
    Reload,
}
struct Ui {
    host: HWND,
    host_identity: isize,
    inline: bool,
    inline_rows: RefCell<Vec<(String, RECT, RECT)>>,
    auxiliary: RefCell<Option<Rc<auxiliary::Open>>>,
    parent: HWND,
    parent_identity: isize,
    hwnd: Cell<HWND>,
    model: RefCell<Rc<model::Model>>,
    bindings: RefCell<Vec<controls::Binding>>,
    placements: RefCell<Vec<(HWND, RECT)>>,
    captions: RefCell<Vec<(HWND, language::Caption)>>,
    language: Cell<crate::ui_text::DisplayLanguage>,
    pending: RefCell<VecDeque<Intent>>,
    closed: Cell<bool>,
    replacing: Cell<bool>,
    page: Cell<usize>,
    scroll: Cell<i32>,
    scroll_x: Cell<i32>,
    content_height: Cell<i32>,
    status: Cell<HWND>,
    status_dirty: Cell<bool>,
    status_writing: Cell<bool>,
    message: RefCell<String>,
    message_caption: RefCell<Option<language::Caption>>,
    capture_windows: Rc<RefCell<Vec<HWND>>>,
}
impl Ui {
    unsafe fn drafts_idle(&self) -> bool {
        let bindings = self.bindings.borrow().clone();
        bindings.iter().all(|binding| match &binding.kind {
            controls::Kind::Text { initial, .. } => {
                !keyboard::composing(binding.hwnd)
                    && controls::text(binding.hwnd).is_some_and(|text| text == *initial)
            }
            _ => true,
        })
    }
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
        self.message_caption.borrow_mut().take();
        self.write_error(message);
    }
    unsafe fn localized_error(&self, id: &'static str) {
        self.error_caption(language::Caption::plain(id));
    }
    unsafe fn error_caption(&self, caption: language::Caption) {
        let message = caption.text();
        *self.message_caption.borrow_mut() = Some(caption);
        self.write_error(&message);
    }
    unsafe fn write_error(&self, message: &str) {
        let changed = *self.message.borrow() != message;
        if changed {
            *self.message.borrow_mut() = message.into();
            self.status_dirty.set(!self.inline);
        }
        if self.inline && changed && !message.is_empty() {
            if let Some(state) = self.state() {
                if let Err(error) = desktop::extension_notices::show(
                    self.host,
                    state,
                    None,
                    crate::ui_text::text("parameter-title"),
                    message,
                ) {
                    eprintln!("native-parameter-notice: {error}");
                }
            }
        }
        self.sync_status();
    }
    unsafe fn sync_status(&self) {
        if self.status_writing.get() {
            return;
        }
        // Let one synchronous update supersede this write, without recursively
        // sending WM_SETTEXT. Any further change remains dirty for the next tick.
        for _ in 0..2 {
            if !self.status_dirty.get() || self.closed.get() {
                return;
            }
            let status = self.status.get();
            if status.is_invalid() || GetParent(status).ok() != Some(self.hwnd.get()) {
                return;
            }
            let text = HSTRING::from(self.message.borrow().as_str());
            self.status_dirty.set(false);
            self.status_writing.set(true);
            let written = SendMessageW(
                status,
                WM_SETTEXT,
                None,
                Some(LPARAM(text.as_ptr() as isize)),
            );
            self.status_writing.set(false);
            if written.0 != 1 {
                if self.status.get() == status {
                    self.status_dirty.set(true);
                }
                return;
            }
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
                    self.localized_error("parameter-complete-input");
                    return;
                }
            }
        };
        let mut pending = self.pending.borrow().clone();
        // Candidates come from established inputs/parameters; do not rebind this transaction's uncommitted draft as new selection eligibility.
        if matches!(&intent, Intent::Candidate(_)) && (!drafts.is_empty() || !pending.is_empty()) {
            self.localized_error("parameter-commit-and-rerun");
            return;
        }
        for item in drafts.into_iter().chain([intent]) {
            transaction::enqueue(&mut pending, item);
        }
        if pending.len() > 4 {
            self.localized_error("parameter-queue-full");
            return;
        }
        if PostMessageW(Some(self.host), WM_NULL, WPARAM(0), LPARAM(0)).is_ok() {
            *self.pending.borrow_mut() = pending;
        } else {
            self.localized_error("parameter-not-queued");
        }
    }
    unsafe fn close(&self) {
        if self.state().is_none_or(|state| state.session_end.paused()) {
            return;
        }
        self.closed.set(true);
        if let Some(state) = self.state() {
            state.parameters_ui.expanded.set(false);
            if self.inline {
                desktop::application::chrome::close_parameter_rows(&state.chrome, self.host);
            }
        }
        self.pending.borrow_mut().clear();
        let auxiliary = self.auxiliary.borrow().clone();
        if let Some(auxiliary) = auxiliary {
            auxiliary.hide();
        }
        let focus = GetFocus();
        let restore = focus == self.hwnd.get() || IsChild(self.hwnd.get(), focus).as_bool();
        let _ = ShowWindow(self.hwnd.get(), SW_HIDE);
        if restore && self.state().is_some() {
            let _ = SetFocus(Some(self.host));
        }
    }
}
pub(in crate::desktop) fn available(state: &State) -> bool {
    !state.session_end.paused()
        && !state.group_hidden.get()
        && !state.group_locked.get()
        && desktop::capture_input::exclusive_available(state)
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
    pub(in crate::desktop) unsafe fn queue_node_action(
        &self,
        stamp: Stamp,
        action: Option<desktop::node_settings::Action>,
    ) {
        self.ui.queue(match action {
            Some(action) => Intent::Setting(stamp, action),
            None => Intent::Run(stamp),
        });
    }
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
        if let Some(state) = self.ui.state() {
            state.parameters_ui.expanded.set(true);
        }
        if enter && self.ui.available() {
            keyboard::enter(&self.ui);
        }
    }
    pub(in crate::desktop) unsafe fn translate(&self, msg: &MSG) -> bool {
        if auxiliary::translate(&self.ui, msg) {
            return true;
        }
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
        empty && self.ui.drafts_idle()
    }
    pub(in crate::desktop) unsafe fn update(&self, state: &State) -> Result<()> {
        if self.retired() {
            return Ok(());
        }
        transaction::drain(self, state)?;
        if self.retired() {
            return Ok(());
        }
        language::refresh(&self.ui, state)?;
        let current = self.ui.model.borrow().current_content(state);
        let waiting = state
            .parameter_view
            .borrow()
            .as_ref()
            .is_some_and(|view| !view.current(state));
        if !current && waiting {
            self.ui.localized_error("parameter-syncing");
        } else if !current {
            if self.ui.drafts_idle() {
                *self.ui.model.borrow_mut() = Rc::new(self.ui.capture(state));
                controls::rebuild(&self.ui)?;
            } else {
                self.ui.localized_error("parameter-config-draft-conflict");
            }
        } else {
            self.ui.sync_status();
        }
        controls::availability(&self.ui);
        Ok(())
    }
    unsafe fn apply(&self, state: &State, intent: Intent) -> Result<()> {
        match intent {
            Intent::Choice(snapshot, index, text) => {
                if !snapshot.current(state) {
                    self.ui.localized_error("parameter-config-changed");
                    return Ok(());
                }
                snapshot.submit_guarded(self.ui.host, state, index, text.as_deref(), &|| {
                    self.ui.available()
                })?;
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
                    self.ui.available() && self.ui.drafts_idle()
                })? {
                    self.ui.localized_error("parameter-candidate-changed");
                } else {
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
                    self.ui.localized_error("parameter-syncing");
                    return Ok(());
                }
                *self.ui.model.borrow_mut() = Rc::new(self.ui.capture(state));
                self.ui.error("");
                controls::rebuild(&self.ui)?;
            }
            Intent::Setting(stamp, action) if stamp.matches(state) => {
                desktop::node_settings::apply(self.ui.host, state, action)?
            }
            Intent::Run(stamp) if stamp.matches(state) => {
                desktop::execution_start::request_after_parameter(self.ui.host, state)?
            }
            Intent::Stop(stamp) if stamp.matches(state) => {
                desktop::execution_cancel::stop(self.ui.host, state)?;
            }
            Intent::Ports(stamp) if stamp.matches(state) => {
                if !desktop::application::chrome::ports::request(&state.chrome, self.ui.host) {
                    self.ui.localized_error("parameter-ports-unavailable");
                }
            }
            Intent::Sticker(stamp, action) => {
                let outcome =
                    desktop::sticker_input::apply(self.ui.host, state, &stamp, action, &|| {
                        self.ui.available()
                    })?;
                self.sticker_outcome(outcome);
            }
            _ => self.ui.localized_error("parameter-config-changed"),
        }
        Ok(())
    }
    unsafe fn sticker_outcome(&self, outcome: desktop::parameter_file_input::Outcome) {
        use desktop::parameter_file_input::Outcome;
        match outcome {
            Outcome::Rejected => self
                .ui
                .localized_error("parameter-image-config-unavailable"),
            Outcome::Cancelled => self.ui.error(""),
            Outcome::Accepted => {
                self.ui.error("");
            }
        }
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        unsafe {
            let hwnd = self.handle();
            if let Some(state) = self.ui.state() {
                if state.parameters_ui.hwnd.get() == hwnd
                    && state.parameters_ui.identity.get() == Rc::as_ptr(&self.ui) as isize
                {
                    if !self.ui.inline {
                        state.parameters_ui.expanded.set(false);
                    }
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
