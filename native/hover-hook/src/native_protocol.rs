//! Scalar protocol for local native button/hover transactions; properties are not authentication, and hosts still check permissions/Destination.
//! Independent epochs distinguish pause/resume and nonces distinguish button transactions; never reuse hover ACTIVE revoked by mouse Leave.
use crate::scope;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

pub const MESSAGE_NAME: PCWSTR = w!("Otter.Live.Native.Input.v3");
pub const EPOCH: PCWSTR = w!("Otter.Live.Native.Epoch.v1");
pub const COMMIT: PCWSTR = w!("Otter.Live.Native.Commit.v1");
pub const COOKIE: PCWSTR = w!("Otter.Live.Native.Cookie.v1");
pub const REQUEST_EPOCH: PCWSTR = w!("Otter.Live.Native.RequestEpoch.v1");
pub const ROOT: PCWSTR = w!("Otter.Live.Native.Root.v1");
pub const DESTINATION: PCWSTR = w!("Otter.Live.Native.Destination.v1");
pub const MESSAGE: PCWSTR = w!("Otter.Live.Native.Message.v1");
pub const WP: PCWSTR = w!("Otter.Live.Native.WParam.v1");
pub const LP: PCWSTR = w!("Otter.Live.Native.LParam.v1");
pub const CLAIMED: PCWSTR = w!("Otter.Live.Native.Claimed.v1");
pub const FINISHED: PCWSTR = w!("Otter.Live.Native.Finished.v1");
pub const REQUEST_KEYS: [PCWSTR; 10] = [
    COMMIT,
    COOKIE,
    REQUEST_EPOCH,
    ROOT,
    DESTINATION,
    MESSAGE,
    WP,
    LP,
    CLAIMED,
    FINISHED,
];

pub fn request_keys() -> impl Iterator<Item = PCWSTR> {
    REQUEST_KEYS
        .into_iter()
        .chain(crate::native_input_scope::keys())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub owner: HWND,
    pub cookie: usize,
    pub epoch: usize,
    pub root: HWND,
    pub destination: HWND,
    pub nonce: usize,
    pub message: u32,
    pub wp: usize,
    pub lp: isize,
    pub popup_scope: crate::native_input_scope::Snapshot,
}

pub fn value(hwnd: HWND, key: PCWSTR) -> usize {
    unsafe { GetPropW(hwnd, key).0 as usize }
}

/// Fixed button/synchronous motion/hover allowlist; no arbitrary Win32 messages or cross-process pointers.
pub fn valid_packet(message: u32, wp: usize, lp: isize) -> bool {
    matches!(
        message,
        WM_LBUTTONDOWN
            | WM_LBUTTONDBLCLK
            | WM_LBUTTONUP
            | WM_RBUTTONUP
            | WM_MOUSEMOVE
            | windows::Win32::UI::Controls::WM_MOUSEHOVER
    ) && wp & !0x7f == 0
        && u32::try_from(lp).is_ok()
}

impl Request {
    fn snapshot(owner: HWND, nonce: usize) -> Option<Self> {
        if nonce == 0 || value(owner, COMMIT) != nonce {
            return None;
        }
        Some(Self {
            owner,
            cookie: value(owner, COOKIE),
            epoch: value(owner, REQUEST_EPOCH),
            root: HWND(value(owner, ROOT) as *mut _),
            destination: HWND(value(owner, DESTINATION) as *mut _),
            nonce,
            message: u32::try_from(value(owner, MESSAGE)).ok()?,
            wp: value(owner, WP),
            lp: value(owner, LP) as isize,
            popup_scope: crate::native_input_scope::Snapshot::read(owner)?,
        })
    }

    /// Read on source message threads; reread commit state at both ends to reject partial publication/revocation/cross-thread targets.
    pub fn read(owner: HWND, nonce: usize, receiving: HWND) -> Option<Self> {
        let request = Self::snapshot(owner, nonce)?;
        if request.destination != receiving
            || request.cookie == 0
            || request.epoch == 0
            || !valid_packet(request.message, request.wp, request.lp)
        {
            return None;
        }
        unsafe {
            if !IsWindowVisible(owner).as_bool()
                || IsIconic(owner).as_bool()
                || value(owner, scope::COOKIE) != request.cookie
                || value(owner, EPOCH) != request.epoch
                || GetAncestor(request.root, GA_ROOT) != request.root
                || GetAncestor(receiving, GA_ROOT) != request.root
                || IsIconic(request.root).as_bool()
            {
                return None;
            }
            let mut root_process = 0;
            let mut destination_process = 0;
            let root_thread = GetWindowThreadProcessId(request.root, Some(&mut root_process));
            let destination_thread =
                GetWindowThreadProcessId(receiving, Some(&mut destination_process));
            if root_process != GetCurrentProcessId()
                || destination_process != root_process
                || root_thread != GetCurrentThreadId()
                || destination_thread != root_thread
            {
                return None;
            }
        }
        (request.popup_scope.valid_for(request.root, owner)
            && Self::snapshot(owner, nonce) == Some(request)
            && value(owner, scope::COOKIE) == request.cookie
            && value(owner, EPOCH) == request.epoch)
            .then_some(request)
    }

    /// Multiple hooks on one source thread must claim before calling applications, preventing duplicate button execution.
    /// Callers still validate nested provenance/save original TLS scope, never overwriting running transactions.
    pub fn claim(self) -> windows::core::Result<bool> {
        if Self::read(self.owner, self.nonce, self.destination) != Some(self)
            || value(self.owner, CLAIMED) == self.nonce
        {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.owner, CLAIMED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        Ok(Self::read(self.owner, self.nonce, self.destination) == Some(self))
    }

    fn same_publication(self) -> bool {
        Self::snapshot(self.owner, self.nonce) == Some(self)
            && value(self.owner, scope::COOKIE) == self.cookie
            && value(self.owner, CLAIMED) == self.nonce
            && value(self.owner, COMMIT) == self.nonce
    }

    /// Called only by dispatch after claim and completed window-procedure execution; not new input authorization.
    /// Clicks may legitimately destroy controls/revoke control; acknowledge only original publication without erasing execution results based on current source state.
    pub(crate) fn finish_returned(self) -> windows::core::Result<bool> {
        if !self.same_publication() {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.owner, FINISHED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        Ok(self.same_publication() && value(self.owner, FINISHED) == self.nonce)
    }
}

#[cfg(test)]
#[path = "native_protocol_tests.rs"]
mod tests;
