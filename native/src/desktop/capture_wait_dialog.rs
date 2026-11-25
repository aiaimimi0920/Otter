//! Modal capture waiting UI; set capture exclusion before starting workers, and keep pumping after cancellation until background resources are released.
use crate::{
    capture_dynamic::{Mode, Outcome},
    capture_job::{CaptureJob, PreparedCapture},
};
use std::cell::{Cell, RefCell};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};
#[path = "capture_wait_hidden.rs"]
mod hidden;
#[path = "capture_wait_layout.rs"]
mod layout;
const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
const TIMER: usize = 0x4f66;
type Factory<T> = Box<dyn FnOnce() -> std::io::Result<CaptureJob<T>>>;
struct Input<T = Outcome> {
    factory: RefCell<Option<Factory<T>>>,
    job: RefCell<Option<CaptureJob<T>>>,
    result: RefCell<Option<Result<Option<T>, String>>>,
    cancelling: Cell<bool>,
    terminal: RefCell<Option<String>>,
    protect: bool,
    quiet: bool,
}
impl<T> Input<T> {
    fn new(factory: Factory<T>, protect: bool) -> Self {
        Self {
            factory: RefCell::new(Some(factory)),
            job: RefCell::new(None),
            result: RefCell::new(None),
            cancelling: Cell::new(false),
            terminal: RefCell::new(None),
            protect,
            quiet: false,
        }
    }
}
/// The caller ensures one active request, hides original windows, and revalidates document/window generations on return; this function publishes no screenshots.
///
/// # Safety
/// owner must be live and belong to the calling thread; call on its UI thread and never destroy stack-allocated modal state during the call.
pub unsafe fn show(
    owner: HWND,
    screen: RECT,
    mode: Mode,
    standard_region: bool,
    gain: f32,
) -> Result<Option<Outcome>, String> {
    let input = Input::new(
        Box::new(move || CaptureJob::start(screen, mode, standard_region, gain)),
        true,
    );
    run(owner, input)
}
/// Snapshot one capture and cache destination; never read mutable settings during modal-message reentrancy.
pub struct CachedRequest {
    pub screen: RECT,
    pub mode: Mode,
    pub standard_region: bool,
    pub gain: f32,
    pub profile: std::path::PathBuf,
    pub stem: String,
    pub policy: crate::cache_policy::Policy,
}
/// Wait for capture and original-image encoding without publishing cache files; callers must revalidate owner after return before commit.
///
/// # Safety
/// As with show: owner is live on the current UI thread, and modal state must not be destroyed during the call.
pub unsafe fn show_cached(
    owner: HWND,
    request: CachedRequest,
) -> Result<Option<PreparedCapture>, String> {
    let quiet = request.standard_region;
    let mut input = Input::new(
        Box::new(move || {
            CaptureJob::start_cached(
                request.screen,
                request.mode,
                request.standard_region,
                request.gain,
                request.profile,
                request.stem,
                request.policy,
            )
        }),
        true,
    );
    input.quiet = quiet;
    run(owner, input)
}
unsafe fn run<T>(owner: HWND, input: Input<T>) -> Result<Option<T>, String> {
    if input.quiet {
        hidden::run(owner, &input)?;
        input.job.borrow_mut().take();
        return input.result.into_inner().unwrap_or(Ok(None));
    }
    let template = layout::template();
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure::<T>),
        LPARAM(&input as *const _ as isize),
    );
    // The task owner still cancels and joins on dialog-creation failure, external destruction, and similar paths.
    input.job.borrow_mut().take();
    if result == -1 {
        return Err(windows::core::Error::from_win32().to_string());
    }
    input.result.into_inner().unwrap_or(Ok(None))
}
unsafe fn finish<T>(hwnd: HWND, input: &Input<T>, result: Result<Option<T>, String>) {
    let _ = KillTimer(Some(hwnd), TIMER);
    input.result.replace(Some(result));
    input.job.borrow_mut().take();
    if !input.quiet {
        let _ = EndDialog(hwnd, 1);
    }
}
unsafe fn poll<T>(hwnd: HWND, input: &Input<T>) {
    let result = input
        .job
        .borrow_mut()
        .as_mut()
        .and_then(CaptureJob::take_ready);
    if let Some(result) = result {
        let result = if let Some(error) = input.terminal.borrow_mut().take() {
            Err(error)
        } else if input.cancelling.get() {
            Ok(None)
        } else {
            result.map(Some)
        };
        finish(hwnd, input, result);
    }
}
unsafe fn cancel<T>(hwnd: HWND, input: &Input<T>, error: Option<&str>) {
    if input.result.borrow().is_some() {
        return;
    }
    input.cancelling.set(true);
    if let Some(error) = error {
        input.terminal.replace(Some(error.to_owned()));
    }
    if let Some(job) = input.job.borrow().as_ref() {
        job.cancel();
    }
    let _ = SetDlgItemTextW(
        hwnd,
        103,
        &HSTRING::from(crate::ui_text::text("capture-cancelling")),
    );
    if let Ok(button) = GetDlgItem(Some(hwnd), 2) {
        let _ = EnableWindow(button, false);
    }
    poll(hwnd, input);
}
unsafe extern "system" fn procedure<T>(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
    }
    let pointer = GetWindowLongPtrW(hwnd, USER) as *const Input<T>;
    let Some(input) = pointer.as_ref() else {
        return 0;
    };
    match message {
        WM_INITDIALOG => {
            if input.protect {
                if let Err(error) = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) {
                    finish(
                        hwnd,
                        input,
                        Err(format!("Cannot exclude capture dialog: {error}")),
                    );
                    return 1;
                }
            }
            let factory = input.factory.borrow_mut().take();
            let Some(factory) = factory else {
                return 1;
            };
            match factory() {
                Ok(job) => {
                    input.job.replace(Some(job));
                }
                Err(error) => {
                    finish(hwnd, input, Err(error.to_string()));
                    return 1;
                }
            }
            // Visible waiting dialogs retain their original timer; ordinary region captures wait directly for the original worker thread.
            if !input.quiet && SetTimer(Some(hwnd), TIMER, 25, None) == 0 {
                finish(
                    hwnd,
                    input,
                    Err("Cannot create capture completion timer".into()),
                );
            }
            1
        }
        WM_TIMER if wp.0 == TIMER => {
            poll(hwnd, input);
            1
        }
        WM_COMMAND if wp.0 & 0xffff == 2 => {
            cancel(hwnd, input, None);
            1
        }
        WM_CLOSE => {
            cancel(hwnd, input, None);
            1
        }
        WM_DISPLAYCHANGE => {
            cancel(hwnd, input, Some("Display layout changed during capture"));
            1
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER);
            input.job.borrow_mut().take();
            SetWindowLongPtrW(hwnd, USER, 0);
            1
        }
        _ => 0,
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_wait_tests.rs"]
mod tests;
