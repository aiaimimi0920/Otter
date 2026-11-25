//! One modeless sharing popup: enqueue only explicit business requests; closing revokes uncommitted results without destroying saved associations.
use super::{Request, State, View};
use crate::workspace_id::UnitId;
const MAX_TEXT: usize = 8192;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "projection_share_account.rs"]
mod account;
#[path = "projection_share_lan.rs"]
mod lan;
#[path = "projection_share_layout.rs"]
mod layout;
#[path = "projection_share_qr.rs"]
mod qr;
#[path = "projection_share_window.rs"]
mod window;
pub(super) use qr::Decode;
pub(in crate::desktop) struct SourceSelection {
    pub unit: UnitId,
    pub origin: String,
    pub target: crate::desktop::popup_menu::target::MenuTarget,
    pub cancel: Arc<AtomicBool>,
}
pub(in crate::desktop) struct Window {
    model: Rc<Model>,
}
struct Model {
    hwnd: Cell<Option<HWND>>,
    owner: HWND,
    view: Weak<RefCell<View>>,
    unit: UnitId,
    origin: String,
    receive: bool,
    public: Cell<bool>,
    envelope: RefCell<Option<crate::projection_delivery::Envelope>>,
    source_loom_id: RefCell<String>,
    account_envelope: RefCell<Option<crate::projection_account::Envelope>>,
    account_import: Cell<bool>,
    create_pending: Cell<bool>,
    target: crate::desktop::popup_menu::target::MenuTarget,
    qr: Cell<bool>,
    input_epoch: Cell<u64>,
    file_dialog_active: Cell<bool>,
    decode_cancel: RefCell<Option<Arc<AtomicBool>>>,
    cancel: Arc<AtomicBool>,
    code: RefCell<Option<qr::Code>>,
    link: RefCell<String>,
    inputs: RefCell<[String; 2]>,
    status: RefCell<String>,
}
pub(in crate::desktop) fn available(state: &State, _receive: bool) -> bool {
    let view = state.authorization.borrow();
    !view.closed && view.origin.is_some() && !state.exit_requested.get() && !state.modals.blocked()
}
fn busy(view: &View) -> bool {
    view.busy
        || view.sender.busy
        || view.projection.busy
        || view.account.busy
        || view.editing.busy
        || view.request.is_some()
}
pub(in crate::desktop) unsafe fn show(
    owner: HWND,
    state: &State,
    receive: bool,
    qr: bool,
) -> Result<()> {
    if !available(state, receive) {
        return Err(Error::new(E_ABORT, "Projection share unavailable"));
    }
    let origin = state
        .authorization
        .borrow()
        .origin
        .clone()
        .ok_or_else(|| Error::new(E_ABORT, "No authorized Loom"))?;
    let existing = state.authorization.borrow().share.clone();
    if let Some(popup) = existing.filter(|p| {
        p.model.owner == owner
            && p.model.unit == state.unit_id
            && p.model.origin == origin
            && p.model.receive == receive
            && p.model.hwnd.get().is_some()
            && p.model.target.matches(&state.canvas.borrow())
    }) {
        popup.model.switch_tab(qr);
        popup.refresh();
        return Ok(());
    }
    let old = state.authorization.borrow_mut().share.take();
    drop(old);
    let cancel = Arc::new(AtomicBool::new(false));
    let model = Rc::new(Model {
        hwnd: Cell::new(None),
        owner,
        view: Rc::downgrade(&state.authorization),
        unit: state.unit_id.clone(),
        origin,
        receive,
        public: Cell::new(false),
        envelope: RefCell::new(None),
        source_loom_id: RefCell::new(String::new()),
        account_envelope: RefCell::new(None),
        account_import: Cell::new(false),
        create_pending: Cell::new(!receive),
        target: crate::desktop::popup_menu::target::MenuTarget::capture(&state.canvas.borrow()),
        qr: Cell::new(qr),
        input_epoch: Cell::new(0),
        file_dialog_active: Cell::new(false),
        decode_cancel: RefCell::new(None),
        cancel: cancel.clone(),
        code: RefCell::new(None),
        link: RefCell::new(String::new()),
        inputs: RefCell::new(Default::default()),
        status: RefCell::new(String::new()),
    });
    window::create(&model)?;
    let popup = Rc::new(Window { model });
    state.authorization.borrow_mut().share = Some(popup.clone());
    popup.refresh();
    Ok(())
}
impl Window {
    pub(in crate::desktop) unsafe fn translate(&self, message: &MSG) -> bool {
        self.model
            .hwnd
            .get()
            .is_some_and(|hwnd| IsDialogMessageW(hwnd, message).as_bool())
    }
    unsafe fn refresh(&self) {
        if self.model.hwnd.get().is_none() || self.model.cancel.load(Ordering::Acquire) {
            return;
        }
        if !self.model.view.upgrade().is_some_and(|v| {
            let v = v.borrow();
            !v.closed && v.origin.as_deref() == Some(&self.model.origin)
        }) {
            if let Some(hwnd) = self.model.hwnd.get() {
                let _ = DestroyWindow(hwnd);
            }
            return;
        }
        if self.model.create_pending.get()
            && self
                .model
                .view
                .upgrade()
                .is_some_and(|v| !busy(&v.borrow()))
        {
            self.model.create_pending.set(false);
            let result = if self.model.public.get() {
                self.model.ensure_account()
            } else if self.model.lan_binding().is_none() {
                self.model.start_lan()
            } else {
                Ok(())
            };
            if let Err(error) = result {
                self.model.set_status(&error.to_string());
            }
        }
        if self.model.public.get() {
            self.model.refresh_account();
        } else {
            self.model.refresh_lan();
        }
    }
}
impl Model {
    unsafe fn switch_tab(&self, qr: bool) {
        if self.account_import.get() || self.qr.replace(qr) == qr {
            return;
        }
        // Both views share the parsed invitation; cancel only unfinished image recognition, without discarding input.
        self.cancel_decode();
        if self.receive {
            let input = self.inputs.borrow()[usize::from(self.public.get())].clone();
            self.parsed(&input);
        }
        if let Some(hwnd) = self.hwnd.get() {
            window::place(self);
            layout::arrange(hwnd, self, windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd));
            for id in [104, 105] {
                if let Ok(button) = GetDlgItem(Some(hwnd), id) {
                    let _ =
                        windows::Win32::Graphics::Gdi::InvalidateRect(Some(button), None, false);
                }
            }
        }
    }
    fn output_current(&self) -> bool {
        if self.public.get() {
            self.account_output_current()
        } else {
            self.lan_output_current()
        }
    }
    unsafe fn set_status(&self, value: &str) {
        if *self.status.borrow() == value {
            return;
        }
        *self.status.borrow_mut() = value.into();
        if let Some(hwnd) = self.hwnd.get() {
            let _ = SetDlgItemTextW(hwnd, 102, &HSTRING::from(value));
            if let Ok(label) = GetDlgItem(Some(hwnd), 102) {
                let _ = ShowWindow(label, if value.is_empty() { SW_HIDE } else { SW_SHOWNA });
            }
            layout::arrange(hwnd, self, windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd));
        }
    }
    fn cancel_decode(&self) {
        self.input_epoch.set(self.input_epoch.get().wrapping_add(1));
        if let Some(signal) = self.decode_cancel.borrow_mut().take() {
            signal.store(true, Ordering::Release);
        }
    }
    unsafe fn parsed(&self, text: &str) {
        if self.receive {
            self.inputs.borrow_mut()[usize::from(self.public.get())] = text.to_owned();
            if let Some(hwnd) = self.hwnd.get() {
                window::input(hwnd, text);
            }
        }
        self.account_envelope.borrow_mut().take();
        self.envelope.borrow_mut().take();
        if !(if self.public.get() {
            self.parse_account(text)
        } else {
            self.parse_lan(text)
        }) {
            self.set_status(if text.trim().is_empty() {
                ""
            } else {
                "无效的投射邀请"
            });
            if let Some(hwnd) = self.hwnd.get() {
                window::enable(hwnd, 1, false);
            }
        }
    }
    unsafe fn submit(&self) -> Result<()> {
        if self.public.get() {
            self.submit_account()
        } else {
            self.submit_lan()
        }
    }
}
pub(in crate::desktop) unsafe fn follow(owner: HWND, state: &State) {
    let popup = state.authorization.borrow().share.clone();
    if let Some(popup) = popup.filter(|p| p.model.owner == owner) {
        window::place(&popup.model);
    }
}
pub(in crate::desktop) unsafe fn refresh(view: &Rc<RefCell<View>>) {
    let done = view
        .borrow_mut()
        .share_decode
        .as_mut()
        .and_then(qr::Decode::poll);
    if let Some((target, epoch, result)) = done {
        view.borrow_mut().share_decode.take();
        if let Some(m) = target.upgrade().filter(|m| {
            !m.cancel.load(Ordering::Acquire) && m.qr.get() && m.input_epoch.get() == epoch
        }) {
            match result {
                Ok(value) => m.parsed(&value),
                Err(error) => m.set_status(error),
            }
        }
    }
    let popup = view.borrow().share.clone();
    if let Some(popup) = popup {
        popup.refresh();
    }
}
pub(in crate::desktop) fn shutdown(view: &Rc<RefCell<View>>) {
    let (popup, decoder) = {
        let mut view = view.borrow_mut();
        (view.share.take(), view.share_decode.take())
    };
    drop(popup);
    // At shutdown join the original thread without UI borrows; closing the popup does not mean the worker has exited.
    drop(decoder);
}
impl Drop for Window {
    fn drop(&mut self) {
        self.model.cancel.store(true, Ordering::Release);
        if let Some(hwnd) = self.model.hwnd.get() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
