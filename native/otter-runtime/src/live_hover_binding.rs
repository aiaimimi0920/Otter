//! Thread-targeted hooks for the input worker; only an absolute-path DLL from the same package, with no global hooks or privilege elevation.
use crate::{
    live_hover_owner::Identity,
    live_native_hooks::{Hooks, Kind},
    live_window_target::Target,
};
use otter_live_hover_hook::scope::*;
use std::sync::Mutex;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

static PROPERTY_WRITE: Mutex<()> = Mutex::new(());
pub struct Binding {
    target: Target,
    owner: Identity,
    hooks: Hooks,
}
impl Binding {
    pub fn close(mut self) -> Result<()> {
        self.detach()
    }
    pub fn new(target: &Target, owner: Identity) -> Result<Self> {
        Self::install(target, owner, false)
    }
    pub fn new_native(target: &Target, owner: Identity) -> Result<Self> {
        Self::install(target, owner, true)
    }
    fn install(target: &Target, owner: Identity, native: bool) -> Result<Self> {
        target.validate()?;
        if !owner.valid() {
            return Err(Error::new(E_ABORT, "Live hover owner closed"));
        }
        let hooks = Hooks::install(target, if native { Kind::Native } else { Kind::Hover })?;
        if !owner.valid() {
            return Err(Error::new(E_ABORT, "Live hover owner closed"));
        }
        Ok(Self {
            target: target.clone(),
            owner,
            hooks,
        })
    }
    pub fn activate(&self) -> Result<()> {
        self.target.validate()?;
        if !self.owner.valid() {
            return Err(Error::new(E_ABORT, "Live hover owner closed"));
        }
        let _guard = PROPERTY_WRITE
            .lock()
            .map_err(|_| Error::new(E_FAIL, "Live hover binding poisoned"))?;
        unsafe {
            let mut cursor = POINT::default();
            if GetPropW(self.owner.window(), ACTIVE) != self.owner.token()
                || GetCursorPos(&mut cursor).is_err()
                || GetAncestor(WindowFromPoint(cursor), GA_ROOT) != self.owner.window()
            {
                return Ok(());
            } // Late samples must not take the filter slot from B, where the mouse currently resides.
              // When multiple Bs share A, the last B actually interacted with gets the filter slot; allow through if the full cookie does not match.
            let root = self.target.hwnd();
            SetPropW(root, SOURCE_COOKIE, Some(HANDLE::default()))?;
            SetPropW(self.owner.window(), SOURCE, Some(HANDLE(root.0)))?;
            SetPropW(root, OWNER, Some(HANDLE(self.owner.window().0)))?;
            SetPropW(root, SOURCE_COOKIE, Some(self.owner.token()))?;
        }
        Ok(())
    }
    fn detach(&mut self) -> Result<()> {
        if self.hooks.closed() {
            return Ok(());
        }
        // If the original Source/Owner identity mismatches, abandon the write; destroying the source HWND automatically removes its properties.
        if let Ok(_guard) = PROPERTY_WRITE.lock() {
            unsafe {
                if self.target.validate().is_ok()
                    && GetPropW(self.target.hwnd(), SOURCE_COOKIE) == self.owner.token()
                {
                    let _ = RemovePropW(self.target.hwnd(), SOURCE_COOKIE);
                    let _ = RemovePropW(self.target.hwnd(), OWNER);
                }
                if self.owner.valid() {
                    let _ = RemovePropW(self.owner.window(), SOURCE);
                }
            }
        }
        self.hooks.detach()
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        if let Err(error) = self.detach() {
            eprintln!("native-live-hover-cleanup: {error}");
        }
    }
}
