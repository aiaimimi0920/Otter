//! The test process centrally owns the document class; individual cases still release their own HWNDs.
use std::{
    cell::{Cell, RefCell},
    sync::Once,
};
use windows::Win32::System::{StationsAndDesktops::*, Threading::GetCurrentThreadId};
use windows::{
    core::{w, Error},
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

/// Isolates native hit tests from unrelated topmost windows without switching the input desktop.
pub(crate) fn with_private_desktop(test: impl FnOnce()) {
    struct Desktop {
        previous: HDESK,
        owned: HDESK,
    }
    impl Desktop {
        fn close(&mut self) -> windows::core::Result<()> {
            if !self.owned.is_invalid() {
                unsafe {
                    SetThreadDesktop(self.previous)?;
                    CloseDesktop(self.owned)?;
                }
                self.owned = HDESK::default();
            }
            Ok(())
        }
    }
    impl Drop for Desktop {
        fn drop(&mut self) {
            let _ = self.close();
        }
    }

    let mut desktop = unsafe {
        let thread = GetCurrentThreadId();
        let previous = GetThreadDesktop(thread).unwrap();
        let name = windows::core::HSTRING::from(format!(
            "Otter.Workspace.HitTest.{}.{thread}",
            std::process::id(),
        ));
        let owned = CreateDesktopW(
            &name,
            None,
            None,
            DESKTOP_CONTROL_FLAGS(0),
            DESKTOP_CREATEWINDOW.0
                | DESKTOP_CREATEMENU.0
                | DESKTOP_READOBJECTS.0
                | DESKTOP_WRITEOBJECTS.0,
            None,
        )
        .unwrap();
        let desktop = Desktop { previous, owned };
        SetThreadDesktop(owned).expect("assign isolated workspace test desktop");
        desktop
    };
    test();
    desktop
        .close()
        .expect("release isolated workspace test desktop");
}

pub(super) unsafe fn register() -> HINSTANCE {
    static REGISTER: Once = Once::new();
    let instance = HINSTANCE::from(GetModuleHandleW(None).unwrap());
    REGISTER.call_once(|| {
        let atom = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            // Preserve actual window-procedure identity so endpoint exclusion distinguishes ordinary documents from Live.
            lpfnWndProc: Some(super::procedure::window_proc),
            ..Default::default()
        });
        assert!(
            atom != 0,
            "register workspace test class: {}",
            Error::from_win32()
        );
    });
    instance
}

thread_local! {
    static CREATE: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static CANDIDATE: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
}

// Inject one reentrancy only when explicitly arranged by the test thread; other messages retain production WndProc.
pub(super) fn during_create(action: impl FnOnce() + 'static) {
    CREATE.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(action));
    });
}

pub(super) fn take_candidate() -> HWND {
    let hwnd = CANDIDATE.with(|slot| slot.replace(HWND::default()));
    assert!(!hwnd.0.is_null(), "WM_NCCREATE hook did not run");
    hwnd
}

pub(super) fn on_create(hwnd: HWND) {
    let action = CREATE.with(|slot| slot.borrow_mut().take());
    if let Some(action) = action {
        CANDIDATE.with(|slot| slot.set(hwnd));
        action();
    }
}
