//! Value-identified local button/hover entry; never infer original mouse-message provenance or replay unconfirmed input.
use crate::native_protocol::{self as protocol, Request};
use std::{cell::Cell, sync::OnceLock};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

thread_local! {
    // Hold neither RefCell borrows nor locks when entering arbitrary/modal application window procedures.
    static CURRENT: Cell<Option<Request>> = const { Cell::new(None) };
    static COLLECTING: Cell<bool> = const { Cell::new(false) };
    static ANCHOR: Cell<Option<crate::native_popup_placement::Anchor>> = const { Cell::new(None) };
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static PUMP: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn place_popup(popup: HWND) -> windows::core::Result<()> {
    match ANCHOR.with(Cell::get) {
        Some(anchor) => anchor.place(popup),
        None => Ok(()),
    }
}

pub fn message_id() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(protocol::MESSAGE_NAME) })
}

/// First message peek/retrieval stops collecting new popups but does not mean the application's button handler returned.
pub fn message_pump_entered() {
    COLLECTING.with(|slot| slot.set(false));
    PUMP.with(|slot| slot.set(slot.get().wrapping_add(1)));
}

pub fn collecting() -> Option<Request> {
    if !COLLECTING.with(Cell::get) {
        return None;
    }
    CURRENT.with(Cell::get).filter(|request| {
        Request::read(request.owner, request.nonce, request.destination) == Some(*request)
    })
}

struct Running {
    request: Request,
    parent: Option<Request>,
    collecting: bool,
    anchor: Option<crate::native_popup_placement::Anchor>,
    pump: usize,
}
impl Drop for Running {
    fn drop(&mut self) {
        COLLECTING.with(|slot| slot.set(false));
        crate::native_popup_layer::finish(self.request);
        CURRENT.with(|slot| slot.set(self.parent));
        ANCHOR.with(|slot| slot.set(self.anchor));
        // If a child transaction enters a message pump, its parent has also crossed the synchronous SHOW provenance boundary.
        COLLECTING.with(|slot| slot.set(self.collecting && PUMP.with(Cell::get) == self.pump));
        DEPTH.with(|slot| slot.set(slot.get() - 1));
    }
}

/// Execute only on the receiver's source thread. Allow only verified menu descendants to nest, independently saving/restoring the original transaction.
/// Windows manages in-flight execution references to remote hook DLLs; host cancellation does not mean this function returned.
pub fn dispatch(owner: HWND, nonce: usize, receiving: HWND) -> windows::core::Result<bool> {
    let Some(request) = Request::read(owner, nonce, receiving) else {
        return Ok(false);
    };
    let parent = CURRENT.with(Cell::get);
    if DEPTH.with(Cell::get) >= 4
        || parent.is_some_and(|parent| !crate::native_nested::allowed(parent, request))
    {
        return Ok(false);
    }
    if !request.claim()? {
        return Ok(false);
    }
    let _running = Running {
        request,
        parent,
        collecting: COLLECTING.with(Cell::get),
        anchor: ANCHOR.with(Cell::get),
        pump: PUMP.with(Cell::get),
    };
    DEPTH.with(|slot| slot.set(slot.get() + 1));
    CURRENT.with(|slot| slot.set(Some(request)));
    COLLECTING.with(|slot| slot.set(true));
    ANCHOR.with(|slot| slot.set(crate::native_popup_placement::Anchor::read(request)));
    crate::native_activation::prepare(request)?;
    // Anchor queries/source-queue activation can reenter applications; accept only original provenance. Claimed is not executed, and rejection never replays.
    if Request::read(owner, nonce, receiving) != Some(request) {
        return Ok(false);
    }
    // request contains only allowlisted buttons/integer parameters; no host addresses or arbitrary message IDs.
    unsafe {
        SendMessageW(
            receiving,
            request.message,
            Some(WPARAM(request.wp)),
            Some(LPARAM(request.lp)),
        );
    }
    // The original window procedure returned; authorization may subsequently revoke, but executed results cannot be misreported as incomplete.
    // finish still matches original cookie/full message/nonce/claim, never acknowledging revoked/replaced publications.
    let _ = request.finish_returned()?;
    Ok(true)
}

/// # Safety
/// Nonnegative Windows WH_CALLWNDPROC code implies valid CWPSTRUCT; always continue the original hook chain.
#[no_mangle]
pub unsafe extern "system" fn otter_live_native_call(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && lp.0 != 0 {
        let call = &*(lp.0 as *const CWPSTRUCT);
        crate::native_popup::before(call);
        let id = message_id();
        if id != 0 && call.message == id {
            // Hosts determine failure by checking CLAIMED/FINISHED; never retry buttons already entered into the application.
            let _ = dispatch(
                HWND(call.wParam.0 as *mut _),
                call.lParam.0 as usize,
                call.hwnd,
            );
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
