//! Fixed scalar menu-z-order requests; independent request HWNDs do not reuse B's input slot, and properties alone are not authentication.
//! Cancellation revokes COMMIT before checking CLAIMED; retain claimed requests until FINISHED, never overwrite/retire early.
use crate::{native_popup::Lease, native_protocol::value};
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

pub const MESSAGE_NAME: PCWSTR = w!("Otter.Live.Popup.Layer.Request.v1");
pub const TOKEN: PCWSTR = w!("Otter.Live.Popup.Layer.Token.v1");
pub const ID: PCWSTR = w!("Otter.Live.Popup.Layer.Id.v1");
pub const COMMIT: PCWSTR = w!("Otter.Live.Popup.Layer.Commit.v1");
pub const OPERATION: PCWSTR = w!("Otter.Live.Popup.Layer.Operation.v1");
pub const POPUP: PCWSTR = w!("Otter.Live.Popup.Layer.Popup.v1");
pub const OWNER: PCWSTR = w!("Otter.Live.Popup.Layer.Owner.v1");
pub const COOKIE: PCWSTR = w!("Otter.Live.Popup.Layer.Cookie.v1");
pub const EPOCH: PCWSTR = w!("Otter.Live.Popup.Layer.Epoch.v1");
pub const ROOT: PCWSTR = w!("Otter.Live.Popup.Layer.Root.v1");
pub const INPUT: PCWSTR = w!("Otter.Live.Popup.Layer.Input.v1");
pub const INSTANCE: PCWSTR = w!("Otter.Live.Popup.Layer.Instance.v1");
pub const CLAIMED: PCWSTR = w!("Otter.Live.Popup.Layer.Claimed.v1");
pub const RESULT: PCWSTR = w!("Otter.Live.Popup.Layer.Result.v1");
pub const FINISHED: PCWSTR = w!("Otter.Live.Popup.Layer.Finished.v1");
pub const PENDING: windows::core::HRESULT = windows::core::HRESULT(0x8000000Au32 as i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Operation {
    Acquire = 1,
    Restore = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub window: HWND,
    pub token: usize,
    pub nonce: usize,
    pub operation: Operation,
    pub lease: Lease,
}
impl Request {
    fn snapshot(window: HWND) -> Option<Self> {
        Some(Self {
            window,
            token: value(window, TOKEN),
            nonce: value(window, ID),
            operation: match value(window, OPERATION) {
                1 => Operation::Acquire,
                2 => Operation::Restore,
                _ => return None,
            },
            lease: Lease {
                popup: HWND(value(window, POPUP) as *mut _),
                owner: HWND(value(window, OWNER) as *mut _),
                cookie: value(window, COOKIE),
                epoch: value(window, EPOCH),
                root: HWND(value(window, ROOT) as *mut _),
                request: value(window, INPUT),
                instance: value(window, INSTANCE),
            },
        })
    }
    /// Compare only immutable request identity; original claimed requests may still receive completion results after COMMIT cancellation.
    pub fn matches(self) -> bool {
        self.token != 0 && self.nonce != 0 && Self::snapshot(self.window) == Some(self)
    }
    /// Source-thread entry; same PID/thread or recovery records alone are not new input permission.
    pub fn read(window: HWND, nonce: usize, receiving: HWND) -> Option<Self> {
        let request = Self::snapshot(window)?;
        let lease = request.lease;
        if nonce == 0
            || request.nonce != nonce
            || request.token == 0
            || value(window, COMMIT) != nonce
            || lease.popup != receiving
            || lease.popup == lease.root
            || lease.instance == 0
            || lease.request == 0
            || lease.cookie == 0
            || lease.epoch == 0
            || lease.owner.0.is_null()
            || lease.root.0.is_null()
            || [lease.popup, lease.owner, lease.root].contains(&window)
        {
            return None;
        }
        unsafe {
            let mut process = 0;
            let thread = GetWindowThreadProcessId(receiving, Some(&mut process));
            if thread != GetCurrentThreadId()
                || process != GetCurrentProcessId()
                || GetAncestor(receiving, GA_ROOT) != receiving
                || GetWindowThreadProcessId(window, None) == 0
            {
                return None;
            }
        }
        (request.matches() && value(window, COMMIT) == nonce).then_some(request)
    }
    pub fn active(self) -> bool {
        Self::read(self.window, self.nonce, self.lease.popup) == Some(self)
    }
    /// Publish CLAIMED before the final COMMIT reread; never mutate source windows unless reread succeeds.
    pub fn claim(self) -> windows::core::Result<bool> {
        if !self.active() || value(self.window, CLAIMED) == self.nonce {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.window, CLAIMED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        if !self.active() {
            self.finish(E_ABORT)?;
            return Ok(false);
        }
        Ok(true)
    }
    pub fn result(self) -> Option<windows::core::HRESULT> {
        // Ordinary polling checks commit markers first; only candidate receipts need full request-identity rereads.
        (value(self.window, FINISHED) == self.nonce
            && value(self.window, CLAIMED) == self.nonce
            && self.matches())
        .then(|| windows::core::HRESULT(value(self.window, RESULT) as u32 as i32))
    }
    /// Write RESULT first and commit FINISHED last; call only after source invocation returns, never treating cancellation as completion.
    pub fn finish(self, result: windows::core::HRESULT) -> windows::core::Result<()> {
        if self.matches() && value(self.window, CLAIMED) == self.nonce {
            unsafe {
                SetPropW(
                    self.window,
                    RESULT,
                    Some(HANDLE(result.0 as u32 as usize as *mut _)),
                )?;
                SetPropW(self.window, FINISHED, Some(HANDLE(self.nonce as *mut _)))?;
                // Wake only the original request window; failed notification does not retract published results, and hosts still retain deadline wakeups.
                if let Err(error) = PostMessageW(Some(self.window), WM_NULL, WPARAM(0), LPARAM(0)) {
                    eprintln!("native-live-popup-layer-notify: {error}");
                }
            }
        }
        Ok(())
    }
}
