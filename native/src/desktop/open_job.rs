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
#[path = "open_capture_ready.rs"]
mod capture_ready;
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
    worker: OpenWorker,
    token: DocumentToken,
}

type DecodeHandle = JoinHandle<std::result::Result<Option<Vec<PreparedDocument>>, String>>;

pub(super) struct OpenWorker {
    handle: Option<DecodeHandle>,
    cancel: Arc<AtomicBool>,
    _permit: crate::edit_worker_budget::Permit<'static>,
}

impl OpenWorker {
    fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub(super) fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
    }
}

impl Drop for OpenWorker {
    fn drop(&mut self) {
        self.cancel();
        // Only a completed worker or a final owner may drop the join handle.
        if let Some(worker) = self.handle.take() {
            let _ = worker.join();
        }
    }
}

pub(super) fn retire(state: &State) -> Option<OpenWorker> {
    let OpenJob { worker, .. } = state.open_job.take()?;
    worker.cancel();
    Some(worker)
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
    start_positioned(hwnd, state, input, new_pin, None)
}
/// 普通截图在原准备线程完成时交付；不等下一次 100 ms 的导入轮询。
pub(super) unsafe fn start_capture_destination(
    hwnd: HWND,
    state: &State,
    new_pin: bool,
    publish: impl FnOnce() -> Result<Input>,
) -> Result<()> {
    check_replace(state, new_pin)?;
    check_pin_slot(state, new_pin)?;
    if state.open_job.borrow().is_some() {
        return Err(Error::new(E_ABORT, "Image loading is already in progress"));
    }
    // publish 只能执行不分派窗口消息的缓存提交；准入到接管任务之间没有模态重入。
    let input = publish()?;
    super::smoke_timing::run("capture-ready", || {
        start_destination(hwnd, state, input, new_pin)?;
        capture_ready::finish(hwnd, state)
    })
}

fn check_replace(state: &State, new_pin: bool) -> Result<()> {
    if !new_pin && !super::open_image::can_replace(state) {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Locked or minified window cannot replace its image",
        ));
    }
    Ok(())
}

fn check_pin_slot(state: &State, new_pin: bool) -> Result<()> {
    if new_pin && state.new_pin.borrow().is_some() {
        return Err(Error::new(
            E_UNEXPECTED,
            "A prepared pin is awaiting window creation",
        ));
    }
    Ok(())
}
pub(super) unsafe fn start_positioned(
    hwnd: HWND,
    state: &State,
    input: Input,
    new_pin: bool,
    position: Option<(POINT, Option<super::clipboard_placement::Request>)>,
) -> Result<()> {
    check_replace(state, new_pin)?;
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
    check_pin_slot(state, new_pin)?;
    if state.open_job.borrow().is_some() {
        return Ok(());
    }
    let canvas = state.canvas.borrow();
    let token = DocumentToken::capture(&canvas);
    let pinned = new_pin || canvas.pinned;
    drop(canvas);
    let worker = start_worker(
        || SetTimer(Some(hwnd), OPEN_TIMER, 100, None) != 0,
        || {
            let _ = KillTimer(Some(hwnd), OPEN_TIMER);
        },
        |worker_cancel| {
            std::thread::Builder::new()
                .name("otter-open".into())
                .spawn(move || {
                    let mut result = load_inputs(input, pinned, &worker_cancel)?;
                    if let (Some(documents), Some((cursor, request))) = (&mut result, position) {
                        for document in documents {
                            document.paste_origin = Some(cursor);
                            document.placement = request.clone();
                        }
                    }
                    Ok(result)
                })
        },
    )?;
    *state.open_job.borrow_mut() = Some(OpenJob {
        new_pin,
        worker,
        token,
    });
    Ok(())
}

fn start_worker(
    reserve_timer: impl FnOnce() -> bool,
    rollback_timer: impl FnOnce(),
    spawn: impl FnOnce(Arc<AtomicBool>) -> std::io::Result<DecodeHandle>,
) -> Result<OpenWorker> {
    let permit = crate::edit_worker_budget::Permit::acquire_open()
        .ok_or_else(|| Error::new(E_ABORT, "Image loading capacity is busy"))?;
    // A failed timer must never leave a decoder for the message thread to join.
    if !reserve_timer() {
        return Err(Error::new(
            E_FAIL,
            "Cannot schedule image loading; original document preserved",
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    match spawn(cancel.clone()) {
        Ok(handle) => Ok(OpenWorker {
            handle: Some(handle),
            cancel,
            _permit: permit,
        }),
        Err(error) => {
            rollback_timer();
            Err(Error::new(E_FAIL, error.to_string()))
        }
    }
}

pub(super) fn cancel(state: &State) {
    if let Some(job) = state.open_job.borrow().as_ref() {
        job.worker.cancel();
    }
}

#[cfg(test)]
pub(super) fn install_gated(
    state: &State,
    new_pin: bool,
    gate: std::sync::mpsc::Receiver<()>,
) -> Arc<AtomicBool> {
    let worker = start_worker(
        || true,
        || unreachable!(),
        |_| {
            std::thread::Builder::new().spawn(move || {
                gate.recv_timeout(std::time::Duration::from_secs(5))
                    .map_err(|error| error.to_string())?;
                let image = super::image::Image {
                    width: 1,
                    height: 1,
                    pixels: vec![255; 4].into(),
                };
                PreparedDocument::from_image(image, new_pin)
                    .map(|document| Some(vec![document]))
                    .map_err(|error| error.to_string())
            })
        },
    )
    .unwrap();
    let cancel = worker.cancel.clone();
    *state.open_job.borrow_mut() = Some(OpenJob {
        new_pin,
        worker,
        token: DocumentToken::capture(&state.canvas.borrow()),
    });
    cancel
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
    let stopped = || cancel.load(Ordering::Acquire);
    if stopped() {
        return Ok(None);
    }
    let _apartment = Apartment::new().map_err(|error| error.to_string())?;
    use super::image::Image;
    let prepare_image = |image| PreparedDocument::from_image_with_cancel(image, pinned, stopped);
    let prepared = match input {
        Input::WithContext(input) => {
            let (captured, origin, context) = *input;
            PreparedDocument::from_capture_with_cancel(
                captured,
                pinned,
                origin,
                Some(context),
                stopped,
            )
        }
        Input::Files(_) => return Err("Batch input requires batch preparation".into()),
        Input::Image(image) => prepare_image(image),
        Input::Editable(content, placement) => content
            .prepare(|| cancel.load(Ordering::Acquire))
            .map(|mut prepared| {
                prepared.placement = placement;
                prepared
            }),
        Input::Captured(captured, origin) => {
            PreparedDocument::from_capture_with_cancel(captured, pinned, origin, None, stopped)
        }
        Input::File(path) => Image::open_with_cancel(&path, &stopped).and_then(prepare_image),
        Input::Encoded(bytes) => {
            Image::from_encoded_with_cancel(&bytes, &stopped).and_then(prepare_image)
        }
        Input::Dib(bytes) => {
            super::clipboard_dib::decode_with_cancel(&bytes, stopped).and_then(prepare_image)
        }
    };
    if stopped() {
        Ok(None)
    } else {
        prepared.map(Some).map_err(|error| error.to_string())
    }
}

pub(super) unsafe fn finish(hwnd: HWND, state: &State, force: bool) {
    let ready = state
        .open_job
        .borrow()
        .as_ref()
        .is_some_and(|job| force || job.worker.is_finished());
    if !ready {
        return;
    }
    let mut job = state.open_job.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), OPEN_TIMER);
    let result = job
        .worker
        .handle
        .take()
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("Image worker failed".into()));
    if job.worker.cancel.load(Ordering::Acquire) || !IsWindow(Some(hwnd)).as_bool() {
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
