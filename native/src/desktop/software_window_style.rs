//! Software-presentation style transactions for owned HWNDs; hold no state that window-procedure reentrancy might borrow during calls.
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};

const MASK: u32 = WS_EX_LAYERED.0;

/// Commit after creation/first submission on the UI thread; failure/unwind restores only this transaction's style bits.
/// Neither detaches DirectComposition nor releases GPU objects; do not use on windows with attached composition trees.
pub struct Transition<'a> {
    hwnd: HWND,
    original: u32,
    desired: u32,
    active: bool,
    owns: Option<&'a dyn Fn() -> bool>,
    _thread: PhantomData<Rc<()>>,
}

impl<'a> Transition<'a> {
    /// # Safety
    /// Callers must own HWND, ensure it is not destroyed/replaced before the transaction ends, and have safely detached the old composition tree.
    /// SetWindowLongPtrW/SetWindowPos reenter window procedures; hold no Canvas/State borrows across these calls.
    pub unsafe fn begin(hwnd: HWND, pinned: bool) -> Result<Self> {
        Self::begin_checked(hwnd, pinned, false, None)
    }
    /// # Safety
    /// The initial HWND must belong to callers; guard validates stable host identity, not merely IsWindow.
    /// Reentrant destruction invalidates guard; stop subsequent writes and never roll back to a potentially reused HWND.
    pub unsafe fn begin_guarded(
        hwnd: HWND,
        pinned: bool,
        owns: &'a dyn Fn() -> bool,
    ) -> Result<Self> {
        Self::begin_checked(hwnd, pinned, false, Some(owns))
    }
    /// # Safety
    /// Call only after the old composition owner is confirmed retired and released, before new GPU attachment; hold no Canvas borrow across calls.
    /// Commit before creating GPU candidates, preventing attachment failure from rolling back to layered and bypassing attachment admission.
    pub unsafe fn gpu_guarded(hwnd: HWND, owns: &'a dyn Fn() -> bool) -> Result<Self> {
        Self::begin_checked(hwnd, false, true, Some(owns))
    }
    unsafe fn begin_checked(
        hwnd: HWND,
        pinned: bool,
        gpu: bool,
        owns: Option<&'a dyn Fn() -> bool>,
    ) -> Result<Self> {
        admitted(owns)?;
        let original = style(hwnd)?;
        // Layered submission does not require removing NOREDIRECTIONBITMAP; ordinary BitBlt still needs a redirection surface.
        // Never change this flag: in-place removal was observed to fail and may partially write other bits.
        if !gpu && !pinned && original & WS_EX_NOREDIRECTIONBITMAP.0 != 0 {
            return Err(Error::new(
                E_INVALIDARG,
                "Software presentation requires a replacement redirection-enabled HWND",
            ));
        }
        let original = original & MASK;
        let desired = if pinned { WS_EX_LAYERED.0 } else { 0 };
        let mut transaction = Self {
            hwnd,
            original,
            desired,
            active: true,
            owns,
            _thread: PhantomData,
        };
        // No style writes means no reentrancy; commit still rechecks styles after ULW.
        if original == desired {
            return Ok(transaction);
        }
        if let Err(error) = replace_bits(hwnd, desired, owns) {
            if let Err(rollback) = transaction.rollback() {
                return Err(Error::new(
                    error.code(),
                    format!("Software window style failed: {error}; rollback={rollback}"),
                ));
            }
            return Err(error);
        }
        Ok(transaction)
    }

    pub fn commit(mut self) -> Result<()> {
        admitted(self.owns)?;
        if unsafe { style(self.hwnd)? } & MASK != self.desired {
            return Err(Error::new(
                E_FAIL,
                "Software window style changed before commit",
            ));
        }
        self.active = false;
        Ok(())
    }

    pub fn removes_layered(&self) -> bool {
        self.original & WS_EX_LAYERED.0 != 0 && self.desired & WS_EX_LAYERED.0 == 0
    }

    pub fn rollback(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        // Roll back once; do not retry the same failure or recursively send window messages from Drop.
        self.active = false;
        if admitted(self.owns).is_err() {
            return Ok(());
        }
        unsafe { replace_bits(self.hwnd, self.original, self.owns) }
    }
}

impl Drop for Transition<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            eprintln!("native-software-style-rollback: {error}");
        }
    }
}

unsafe fn style(hwnd: HWND) -> Result<u32> {
    if !IsWindow(Some(hwnd)).as_bool()
        || GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId()
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Software style requires an owned UI-thread HWND",
        ));
    }
    SetLastError(WIN32_ERROR(0));
    let value = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if value == 0 && GetLastError() != WIN32_ERROR(0) {
        return Err(Error::from_win32());
    }
    Ok(value as u32)
}

fn admitted(owns: Option<&dyn Fn() -> bool>) -> Result<()> {
    if owns.is_none_or(|owns| owns()) {
        Ok(())
    } else {
        Err(Error::new(E_ABORT, "Software style owner was revoked"))
    }
}
unsafe fn replace_bits(hwnd: HWND, bits: u32, owns: Option<&dyn Fn() -> bool>) -> Result<()> {
    admitted(owns)?;
    let current = style(hwnd)?;
    let next = (current & !MASK) | bits;
    if current != next {
        SetLastError(WIN32_ERROR(0));
        if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next as isize) == 0
            && GetLastError() != WIN32_ERROR(0)
        {
            return Err(Error::from_win32());
        }
        admitted(owns)?;
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )?;
        admitted(owns)?;
    }
    if style(hwnd)? & MASK != bits {
        return Err(Error::new(
            E_FAIL,
            "Software window style mutation was not retained",
        ));
    }
    Ok(())
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "software_window_style_tests.rs"]
mod tests;
