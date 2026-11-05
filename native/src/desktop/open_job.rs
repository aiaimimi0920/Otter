//! 单个后台解码任务；COM 不跨线程，结果提交校验文档身份/版本，空闲时无轮询。
use super::{canvas::Canvas, open_image::PreparedDocument, Apartment, State};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
use windows::{
    core::{w, Error, Result, HSTRING, PCWSTR},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};

pub(super) const OPEN_TIMER: usize = 0x4F55;
#[path = "open_batch.rs"]
mod batch;
use batch::load_inputs;
pub(super) const MAX_BATCH: usize = 32;
pub(super) enum Input {
    File(PathBuf),
    Files(Vec<PathBuf>),
    Encoded(crate::pixels::Pixels),
    Dib(crate::pixels::Pixels),
    Image(super::image::Image),
    Editable(
        Arc<super::clipboard_editable::Content>,
        Option<super::clipboard_placement::Request>,
    ),
    Captured(crate::capture_job::Captured, Option<POINT>),
    WithContext(
        Box<(
            crate::capture_job::Captured,
            Option<POINT>,
            crate::capture_job::Context,
        )>,
    ),
}

struct DocumentToken {
    stamp: crate::editor::DocumentStamp,
    pixels: Option<crate::pixels::Pixels>,
}
impl DocumentToken {
    fn capture(canvas: &Canvas) -> Self {
        Self {
            stamp: canvas.editor.document_stamp(),
            pixels: canvas.image.as_ref().map(|image| image.pixels.clone()),
        }
    }
    fn matches(&self, canvas: &Canvas) -> bool {
        self.stamp.matches(&canvas.editor)
            && canvas.minified.is_none()
            && canvas.editor.draft().is_none()
            && canvas.selection_drag.is_none()
            && canvas.marquee.is_none()
            && canvas.path_drag.is_none()
            && canvas.erase_gesture.is_none()
            && match (&self.pixels, &canvas.image) {
                (None, None) => true,
                (Some(pixels), Some(image)) => pixels.same_storage(&image.pixels),
                _ => false,
            }
    }
}

pub(super) struct OpenJob {
    new_pin: bool,
    worker: Option<JoinHandle<std::result::Result<Option<Vec<PreparedDocument>>, String>>>,
    cancel: Arc<AtomicBool>,
    token: DocumentToken,
}
impl Drop for OpenJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // WIC/文件系统调用不保证可中断；退出仍等待当前调用返回，不遗留无 owner 的线程。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) unsafe fn start_image(
    hwnd: HWND,
    state: &State,
    image: super::image::Image,
    origin: Option<POINT>,
) -> Result<()> {
    start_input(hwnd, state, capture_input(image, origin))
}
unsafe fn start_input(hwnd: HWND, state: &State, input: Input) -> Result<()> {
    start_destination(hwnd, state, input, false)
}
pub(super) unsafe fn start_pin(
    hwnd: HWND,
    state: &State,
    image: super::image::Image,
    origin: Option<POINT>,
) -> Result<()> {
    start_destination(hwnd, state, capture_input(image, origin), true)
}
pub(super) unsafe fn start_long(
    hwnd: HWND,
    state: &State,
    image: super::image::Image,
    origin: POINT,
    axis: crate::long_capture::Axis,
    size: (u32, u32),
) -> Result<()> {
    let mut context = crate::capture_job::Context::region(Some(origin), size.0, size.1, "gdi");
    context.kind = crate::session_capture::Kind::Long;
    context.axis = Some(match axis {
        crate::long_capture::Axis::Horizontal => crate::session_capture::Axis::Horizontal,
        crate::long_capture::Axis::Vertical => crate::session_capture::Axis::Vertical,
    });
    start_long_context(hwnd, state, image, origin, context)
}
pub(super) unsafe fn start_long_context(
    hwnd: HWND,
    state: &State,
    image: super::image::Image,
    origin: POINT,
    context: crate::capture_job::Context,
) -> Result<()> {
    // 新自动录制与既有显式轴共享提交路径，来源扩展不能伪造已确认轴或超限帧数。
    if context.kind != crate::session_capture::Kind::Long
        || crate::session_capture::decode(&context.metadata(false, false).to_string()).is_err()
    {
        return Err(Error::new(E_INVALIDARG, "Invalid long capture metadata"));
    }
    let Input::Captured(captured, origin) = capture_input(image, Some(origin)) else {
        unreachable!()
    };
    start_destination(
        hwnd,
        state,
        Input::WithContext(Box::new((captured, origin, context))),
        true,
    )
}
fn capture_input(image: super::image::Image, origin: Option<POINT>) -> Input {
    Input::Captured(
        crate::capture_job::Captured {
            backend: "gdi",
            original_lease: None,
            preview: crate::capture_color::SdrFrame {
                width: image.width,
                height: image.height,
                pixels: image.pixels,
            },
            original: None,
            reason: None,
            downgraded: false,
            display: None,
            diagnostic: None,
        },
        origin,
    )
}
pub(super) unsafe fn start_destination(
    hwnd: HWND,
    state: &State,
    input: Input,
    new_pin: bool,
) -> Result<()> {
    if !new_pin && !super::open_image::can_replace(state) {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Locked or minified window cannot replace its image",
        ));
    }
    if !new_pin && matches!(&input, Input::Editable(..)) {
        return Err(Error::new(
            E_INVALIDARG,
            "Editable paste requires a new pin",
        ));
    }
    if let Input::Files(paths) = &input {
        if !new_pin || paths.is_empty() || paths.len() > MAX_BATCH {
            return Err(Error::new(E_INVALIDARG, "Invalid image import batch"));
        }
    }
    if new_pin && state.new_pin.borrow().is_some() {
        return Err(Error::new(
            E_UNEXPECTED,
            "A prepared pin is awaiting window creation",
        ));
    }
    if state.open_job.borrow().is_some() {
        return Ok(());
    }
    let canvas = state.canvas.borrow();
    let token = DocumentToken::capture(&canvas);
    let pinned = new_pin || canvas.pinned;
    drop(canvas);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker = std::thread::Builder::new()
        .name("otter-open".into())
        .spawn(move || load_inputs(input, pinned, &worker_cancel))
        .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    *state.open_job.borrow_mut() = Some(OpenJob {
        new_pin,
        worker: Some(worker),
        cancel,
        token,
    });
    scheduled(
        &state.open_job,
        SetTimer(Some(hwnd), OPEN_TIMER, 100, None) != 0,
    )
}

fn scheduled(slot: &std::cell::RefCell<Option<OpenJob>>, timer_created: bool) -> Result<()> {
    if timer_created {
        return Ok(());
    }
    // 调度失败绝不能退化为同步提交。先释放 RefCell 借用，再由 owner 取消并回收线程。
    let job = slot.borrow_mut().take();
    drop(job);
    Err(Error::new(
        E_FAIL,
        "Cannot schedule image loading; original document preserved",
    ))
}

pub(super) fn cancel(state: &State) {
    if let Some(job) = state.open_job.borrow().as_ref() {
        job.cancel.store(true, Ordering::Release);
    }
}

#[cfg(test)]
fn load(
    path: PathBuf,
    pinned: bool,
    cancel: &AtomicBool,
) -> std::result::Result<Option<PreparedDocument>, String> {
    load_input(Input::File(path), pinned, cancel)
}
fn load_input(
    input: Input,
    pinned: bool,
    cancel: &AtomicBool,
) -> std::result::Result<Option<PreparedDocument>, String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    let _apartment = Apartment::new().map_err(|error| error.to_string())?;
    let prepared = match input {
        Input::WithContext(input) => {
            let (captured, origin, context) = *input;
            let metadata = context.metadata(captured.original.is_some(), captured.downgraded);
            PreparedDocument::from_capture(captured, pinned, origin).map(|mut document| {
                if let Some(source) = document.capture_source.as_mut() {
                    source.session_meta = Some(metadata);
                }
                document
            })
        }
        Input::Files(_) => return Err("Batch input requires batch preparation".into()),
        Input::Image(image) => PreparedDocument::from_image(image, pinned),
        Input::Editable(content, placement) => content
            .prepare(|| cancel.load(Ordering::Acquire))
            .map(|mut prepared| {
                prepared.placement = placement;
                prepared
            }),
        Input::Captured(captured, origin) => {
            PreparedDocument::from_capture(captured, pinned, origin)
        }
        Input::File(path) => PreparedDocument::open(&path, pinned),
        Input::Encoded(bytes) => super::image::Image::from_encoded(&bytes)
            .and_then(|image| PreparedDocument::from_image(image, pinned)),
        Input::Dib(bytes) => {
            super::clipboard_dib::decode_with_cancel(&bytes, || cancel.load(Ordering::Acquire))
                .and_then(|image| PreparedDocument::from_image(image, pinned))
        }
    }
    .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Acquire) {
        Ok(None)
    } else {
        Ok(Some(prepared))
    }
}

pub(super) unsafe fn finish(hwnd: HWND, state: &State, force: bool) {
    let ready = state.open_job.borrow().as_ref().is_some_and(|job| {
        job.worker
            .as_ref()
            .is_some_and(|worker| force || worker.is_finished())
    });
    if !ready {
        return;
    }
    let mut job = state.open_job.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), OPEN_TIMER);
    let result = job
        .worker
        .take()
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("Image worker failed".into()));
    if job.cancel.load(Ordering::Acquire) || !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let mut prepared = match result {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return,
        Err(error) => {
            report(hwnd, &format!("无法打开图片，原文档未被替换。\n{error}"));
            return;
        }
    };
    if job.new_pin {
        *state.new_pin.borrow_mut() = Some(prepared);
        return;
    }
    if prepared.len() != 1 {
        report(hwnd, "替换图片必须恰好包含一张图片，原文档保留。");
        return;
    }
    let prepared = prepared.pop().unwrap();
    if prepared.editable.is_some() {
        report(hwnd, "Editable content cannot replace the current image");
        return;
    }
    if !job.token.matches(&state.canvas.borrow()) {
        report(hwnd, "加载期间文档已修改，已取消替换。请重新打开图片。");
        return;
    }
    let captured = prepared.record_history;
    let replaced = {
        let mut canvas = state.canvas.borrow_mut();
        canvas
            .replace_image(prepared.image)
            .map(|()| canvas.attach_capture(prepared.capture_source))
    };
    if let Err(error) = replaced {
        report(hwnd, &format!("无法打开图片，原文档未被替换。\n{error}"));
        return;
    }
    *state.hit_region.borrow_mut() = prepared.region;
    if captured {
        super::capture_history_ui::record(state);
    }
    if state.canvas.borrow().pinned {
        super::pin_window::fit(hwnd, state);
    }
    if let Err(error) = super::hit_region::update(hwnd, state) {
        *state.error.borrow_mut() = Some(error);
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        return;
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn report(hwnd: HWND, message: &str) {
    let message = HSTRING::from(message);
    MessageBoxW(
        Some(hwnd),
        PCWSTR(message.as_ptr()),
        w!("Otter"),
        MB_OK | MB_ICONWARNING,
    );
}

#[cfg(test)]
#[path = "open_job_tests.rs"]
mod tests;
