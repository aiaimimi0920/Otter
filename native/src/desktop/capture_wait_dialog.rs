//! 模态截图等待界面；先设置捕获排除再启动 worker，取消保持泵消息直至后台资源释放。
use crate::{
    capture_dynamic::{Mode, Outcome},
    capture_job::{CaptureJob, PreparedCapture},
};
use std::cell::{Cell, RefCell};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};
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
        }
    }
}
/// 调用方负责单活动请求、隐藏原窗口及返回后的文档/窗口代次验证；本函数不发布截图。
///
/// # Safety
/// owner 必须属于调用线程且存活；在其 UI 线程调用，不得在调用期间销毁栈上的模态状态。
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
/// 单次捕获及缓存目的地快照；不得在模态消息重入时读取可变设置。
pub struct CachedRequest {
    pub screen: RECT,
    pub mode: Mode,
    pub standard_region: bool,
    pub gain: f32,
    pub profile: std::path::PathBuf,
    pub stem: String,
    pub policy: crate::cache_policy::Policy,
}
/// 等待采集和原图编码，但不发布缓存文件；返回后调用方须复核 owner 再 commit。
///
/// # Safety
/// 与 show 相同：owner 属于当前 UI 线程且存活，模态状态不得在调用中被销毁。
pub unsafe fn show_cached(
    owner: HWND,
    request: CachedRequest,
) -> Result<Option<PreparedCapture>, String> {
    let input = Input::new(
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
    run(owner, input)
}
unsafe fn run<T>(owner: HWND, input: Input<T>) -> Result<Option<T>, String> {
    let template = layout::template();
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure::<T>),
        LPARAM(&input as *const _ as isize),
    );
    // 对话框创建失败、外部销毁等路径仍由任务 owner 取消并 join。
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
    let _ = EndDialog(hwnd, 1);
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
    let _ = SetDlgItemTextW(hwnd, 103, w!("正在取消…"));
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
            if SetTimer(Some(hwnd), TIMER, 25, None) == 0 {
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
#[cfg(test)]
#[path = "capture_wait_tests.rs"]
mod tests;
