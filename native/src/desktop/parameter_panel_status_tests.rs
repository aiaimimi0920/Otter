//! Actual hidden STATIC children: verify writes, rebuilds, and WM_SETTEXT reentrancy.
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

struct Probe<'a> {
    ui: &'a Ui,
    attempts: Cell<usize>,
    reject: Cell<bool>,
    replace_text: Cell<bool>,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(observe), id);
    } else if message == WM_SETTEXT {
        let probe = &*(data as *const Probe<'_>);
        probe.attempts.set(probe.attempts.get() + 1);
        if probe.reject.replace(false) {
            return LRESULT(0);
        }
        if probe.replace_text.replace(false) {
            // Enter the owner before the original WM_SETTEXT consumes its buffer.
            probe.ui.error("较新的状态");
        }
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
impl<'a> Probe<'a> {
    fn new(ui: &'a Ui) -> Self {
        Self {
            ui,
            attempts: Cell::default(),
            reject: Cell::default(),
            replace_text: Cell::default(),
        }
    }
    unsafe fn install(&self) -> HWND {
        let hwnd = self.ui.status.get();
        assert!(SetWindowSubclass(hwnd, Some(observe), 1, self as *const _ as usize).as_bool());
        hwnd
    }
}
impl Drop for Probe<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.ui.status.get(), Some(observe), 1);
        }
    }
}
#[test]
fn parameter_status_retries_rejected_write_then_skips_unchanged_and_rebuilt_labels() {
    unsafe {
        let f = Fixture::new();
        let probe = Probe::new(&f.panel.ui);
        let old = probe.install();
        probe.reject.set(true);
        f.panel.ui.error("参数同步中");
        assert_eq!(controls::text(old).as_deref(), Some(""));
        assert_eq!(probe.attempts.get(), 1);
        // Retry from the next owner update even if no branch repeats the error.
        f.tick();
        assert_eq!(controls::text(old).as_deref(), Some("参数同步中"));
        f.panel.ui.error("参数同步中");
        assert_eq!(probe.attempts.get(), 2);
        controls::rebuild(&f.panel.ui).unwrap();
        let rebuilt = probe.install();
        assert_eq!(controls::text(rebuilt).as_deref(), Some("参数同步中"));
        f.panel.ui.error("参数同步中");
        assert_eq!(probe.attempts.get(), 2);
        f.panel.ui.error("");
        assert_eq!(controls::text(rebuilt).as_deref(), Some(""));
        assert_eq!(probe.attempts.get(), 3);
    }
}
#[test]
fn parameter_status_synchronous_reentry_keeps_newest_message_visible() {
    unsafe {
        let f = Fixture::new();
        let probe = Probe::new(&f.panel.ui);
        let hwnd = probe.install();
        probe.replace_text.set(true);
        f.panel.ui.error("旧状态");
        assert_eq!(controls::text(hwnd).as_deref(), Some("较新的状态"));
        f.panel.ui.error("较新的状态");
        assert_eq!(probe.attempts.get(), 2);
    }
}
