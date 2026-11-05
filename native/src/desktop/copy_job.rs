//! 显式复制命令的单任务图像准备；取消或文档变化时禁止迟到结果覆盖系统剪贴板。
use super::{canvas::Canvas, State};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) const TIMER: usize = 0x4F5C;
pub(super) const COMMAND: usize = 0x1040;
pub(super) type Authority = std::rc::Rc<std::cell::RefCell<Arc<AtomicBool>>>;
#[path = "copy_file_plan.rs"]
mod file_plan;
type Output = Result<Option<Ready>, String>;
struct Ready {
    payload: super::clipboard_bitmap::Payload,
    cache: Option<crate::clipboard_cache::Prepared>,
    editable: Option<Arc<super::clipboard_editable::Content>>,
}
pub(super) struct Job {
    worker: Option<JoinHandle<Output>>,
    cancel: Arc<AtomicBool>,
    stamp: super::display_snapshot::Stamp,
    sequence: Option<u32>,
    preserve_edits: bool,
    origin: Option<super::clipboard_placement::Origin>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // WIC 编码不可中断；关闭仍 join，不能遗留无 owner 的任务。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
fn prepare(
    canvas: &Canvas,
    frame: Option<&super::display_frame::Frame>,
) -> Result<Option<Job>, String> {
    prepare_with_plan(canvas, frame, None)
}
fn prepare_with_plan(
    canvas: &Canvas,
    frame: Option<&super::display_frame::Frame>,
    plan: Option<file_plan::Plan>,
) -> Result<Option<Job>, String> {
    let Some((image, editor)) = super::display_snapshot::capture(canvas, frame) else {
        return Ok(None);
    };
    let stamp = super::display_snapshot::Stamp::capture(canvas, frame);
    let opacity = canvas.normal_opacity;
    let preserve_edits = canvas.minified.is_none();
    // Resolved pixels belong to their display source, not this canvas's old capture.
    let capture_source = (preserve_edits && frame.is_none())
        .then(|| canvas.capture_source().cloned())
        .flatten();
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let worker = std::thread::Builder::new()
        .name("otter-copy".into())
        .spawn(move || {
            if signal.load(Ordering::Acquire) {
                return Ok(None);
            }
            let result = super::export::clipboard_payload_with_opacity(&image, &editor, opacity)
                .map_err(|error| error.to_string());
            if signal.load(Ordering::Acquire) {
                Ok(None)
            } else {
                let payload = result?;
                let cache = plan
                    .map(|plan| plan.stage(payload.png.clone(), &signal))
                    .transpose()?;
                if signal.load(Ordering::Acquire) {
                    Ok(None)
                } else {
                    let editable = preserve_edits.then(|| {
                        Arc::new(super::clipboard_editable::Content {
                            image,
                            editor,
                            opacity,
                            capture_source,
                            origin: None,
                        })
                    });
                    Ok(Some(Ready {
                        payload,
                        cache,
                        editable,
                    }))
                }
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(Some(Job {
        worker: Some(worker),
        cancel,
        stamp,
        sequence: None,
        preserve_edits,
        origin: None,
    }))
}
fn eligible(canvas: &Canvas, frame: Option<&super::display_frame::Frame>, job: &Job) -> bool {
    !job.cancel.load(Ordering::Acquire)
        && job.stamp.matches(canvas, frame)
        && canvas.editor.draft().is_none()
        && canvas.selection_drag.is_none()
        && canvas.marquee.is_none()
        && canvas.path_drag.is_none()
        && canvas.erase_gesture.is_none()
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> windows::core::Result<()> {
    let enabled = (state.resolved_display.borrow().is_some()
        || state.canvas.borrow().image.is_some())
        && state.copy_job.borrow().is_none();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        w!("复制图片\tCtrl+C"),
    )
}
pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    if state.copy_job.borrow().is_some()
        || state.modals.blocked()
        || (state.resolved_display.borrow().is_none() && state.canvas.borrow().image.is_none())
    {
        return;
    }
    let sequence = windows::Win32::System::DataExchange::GetClipboardSequenceNumber();
    match file_plan::Plan::from_state(state) {
        Ok(plan) => start_guarded(hwnd, state, Some(plan), Some(sequence)),
        Err(error) => report(hwnd, &error),
    }
}
pub(super) unsafe fn start_with_plan(hwnd: HWND, state: &State, plan: Option<file_plan::Plan>) {
    start_guarded(hwnd, state, plan, None);
}
unsafe fn start_guarded(
    hwnd: HWND,
    state: &State,
    plan: Option<file_plan::Plan>,
    sequence: Option<u32>,
) {
    if state.copy_job.borrow().is_some() || state.file_dialog.get() {
        return;
    }
    state.canvas.borrow_mut().cancel_gesture();
    let prepared = prepare_with_plan(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
        plan,
    );
    match prepared {
        Ok(mut job) => {
            if let Some(job) = &mut job {
                job.sequence = sequence;
                job.preserve_edits &= !state.is_art();
                job.origin = super::clipboard_placement::Origin::capture(hwnd, state);
                // 跨窗口取消旧 worker；旧结果仍由原窗口回收，不转移 HWND 或 State。
                state
                    .copy_authority
                    .replace(job.cancel.clone())
                    .store(true, Ordering::Release);
            }
            *state.copy_job.borrow_mut() = job;
        }
        Err(error) => {
            report(hwnd, &error);
            return;
        }
    }
    if state.copy_job.borrow().is_some() && SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        let job = state.copy_job.borrow_mut().take();
        drop(job);
        report(hwnd, "无法调度复制任务，剪贴板未修改。");
    }
}
pub(super) fn cancel(state: &State) {
    if let Some(job) = state.copy_job.borrow().as_ref() {
        job.cancel.store(true, Ordering::Release);
    }
}
pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    poll_guarded(
        hwnd,
        state,
        || windows::Win32::System::DataExchange::GetClipboardSequenceNumber(),
        |owner, payload, paths, sequence| {
            super::clipboard::copy_formats_checked(owner, payload, paths, sequence)
        },
    );
}

// 测试仅替换最终外部副作用；任务回收、取消和文档校验与生产路径完全共用。
pub(super) unsafe fn poll_using(
    hwnd: HWND,
    state: &State,
    publish: impl FnOnce(HWND, &super::clipboard_bitmap::Payload) -> windows::core::Result<()>,
) {
    poll_files_using(hwnd, state, |owner, payload, _| publish(owner, payload));
}

unsafe fn poll_files_using(
    hwnd: HWND,
    state: &State,
    publish: impl FnOnce(
        HWND,
        &super::clipboard_bitmap::Payload,
        &[std::path::PathBuf],
    ) -> windows::core::Result<()>,
) {
    poll_guarded(
        hwnd,
        state,
        || 0,
        |owner, payload, paths, _| publish(owner, payload, paths).map(|()| None),
    );
}
unsafe fn poll_guarded(
    hwnd: HWND,
    state: &State,
    current_sequence: impl FnOnce() -> u32,
    publish: impl FnOnce(
        HWND,
        &super::clipboard_bitmap::Payload,
        &[std::path::PathBuf],
        Option<u32>,
    ) -> windows::core::Result<Option<u32>>,
) {
    if !state.copy_job.borrow().as_ref().is_some_and(|job| {
        job.worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
    }) {
        return;
    }
    let mut job = state.copy_job.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), TIMER);
    let result = job
        .worker
        .take()
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("Copy worker failed".into()));
    if !IsWindow(Some(hwnd)).as_bool()
        || job
            .sequence
            .is_some_and(|expected| current_sequence() != expected)
        || !eligible(
            &state.canvas.borrow(),
            state.resolved_display.borrow().as_ref(),
            &job,
        )
    {
        return;
    }
    match result {
        Ok(Some(mut ready)) => {
            if let Some(content) = ready.editable.as_mut().and_then(Arc::get_mut) {
                content.origin = job.origin.take();
            }
            let paths = match ready
                .cache
                .map(|cache| cache.commit(&job.cancel))
                .transpose()
            {
                Ok(path) => path.into_iter().collect::<Vec<_>>(),
                Err(error) => {
                    report(hwnd, &error.to_string());
                    return;
                }
            };
            // 一旦开始发布，失败可能已转移部分格式；保留 backing file，不能在此删除。
            match publish(hwnd, &ready.payload, &paths, job.sequence) {
                Ok(Some(sequence)) => state.editable_clipboard.borrow_mut().published(
                    sequence,
                    if job.preserve_edits {
                        ready.editable
                    } else {
                        None
                    },
                ),
                Ok(None) => {} // Final-lock sequence mismatch: nothing was published.
                Err(error) => report(hwnd, &error.to_string()),
            }
        }
        Ok(None) => {}
        Err(error) => report(hwnd, &error),
    }
}
unsafe fn report(hwnd: HWND, message: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("复制未完成。\n{message}")),
        w!("Otter"),
        MB_OK | MB_ICONWARNING,
    );
}

#[cfg(test)]
#[path = "copy_file_tests.rs"]
mod file_tests;
#[cfg(test)]
#[path = "copy_job_tests.rs"]
mod tests;
