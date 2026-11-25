//! Single-task image preparation for explicit copy commands; cancellation/document changes prohibit late results from overwriting the system clipboard.
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
    _permit: crate::edit_worker_budget::Permit<'static>,
    worker: Option<JoinHandle<Output>>,
    cancel: Arc<AtomicBool>,
    stamp: super::display_snapshot::Stamp,
    sequence: Option<u32>,
    preserve_edits: bool,
    origin: Option<super::clipboard_placement::Origin>,
    next: Option<Next>,
}
impl Job {
    pub(super) fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(next) = &self.next {
            next.cancel.store(true, Ordering::Release);
        }
    }

    pub(super) fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

#[cfg(test)]
pub(super) fn install_gated(
    state: &State,
    release: std::sync::mpsc::Receiver<()>,
    returned: std::sync::mpsc::Sender<bool>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    *state.copy_job.borrow_mut() = Some(Job {
        _permit: crate::edit_worker_budget::Permit::acquire_output().unwrap(),
        worker: Some(std::thread::spawn(move || {
            let _ = release.recv_timeout(std::time::Duration::from_secs(5));
            returned
                .send(worker_cancel.load(Ordering::Acquire))
                .unwrap();
            Ok(None)
        })),
        cancel,
        stamp: super::display_snapshot::Stamp::capture(&state.canvas.borrow(), None),
        sequence: None,
        preserve_edits: false,
        origin: None,
        next: None,
    });
}
// New explicit copy requests occupy at most one successor slot; reclaim old encoders first, without key-callback joins or parallel encoding.
struct Next {
    stamp: super::display_snapshot::Stamp,
    plan: Option<file_plan::Plan>,
    sequence: Option<u32>,
    origin: Option<super::clipboard_placement::Origin>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
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
    let permit = crate::edit_worker_budget::Permit::acquire_output()
        .ok_or("Native output worker limit reached")?;
    let stamp = super::display_snapshot::Stamp::capture(canvas, frame);
    let opacity = canvas.normal_opacity;
    let style = super::export::style::Snapshot::capture(canvas);
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
            let result = style
                .clipboard(&image, &editor, opacity)
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
                            frame_style: style.style,
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
        _permit: permit,
        worker: Some(worker),
        cancel,
        stamp,
        sequence: None,
        preserve_edits,
        origin: None,
        next: None,
    }))
}
fn eligible(canvas: &Canvas, frame: Option<&super::display_frame::Frame>, job: &Job) -> bool {
    !job.cancel.load(Ordering::Acquire) && job.stamp.matches(canvas, frame) && gesture_idle(canvas)
}
fn gesture_idle(canvas: &Canvas) -> bool {
    canvas.editor.draft().is_none()
        && canvas.selection_drag.is_none()
        && canvas.marquee.is_none()
        && canvas.path_drag.is_none()
        && canvas.erase_gesture.is_none()
}
pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    if state.modals.blocked()
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
    // Sample before preparing pixels/starting workers, matching the pointer timing of the baseline copy entry.
    let origin = super::clipboard_placement::Origin::capture(hwnd, state);
    start_guarded_at(hwnd, state, plan, sequence, origin);
}
pub(super) unsafe fn start_guarded_at(
    hwnd: HWND,
    state: &State,
    plan: Option<file_plan::Plan>,
    sequence: Option<u32>,
    origin: Option<super::clipboard_placement::Origin>,
) {
    if state.file_dialog.get() {
        return;
    }
    state.canvas.borrow_mut().cancel_gesture();
    if let Some(job) = state.copy_job.borrow_mut().as_mut() {
        if job.sequence == sequence
            && eligible(
                &state.canvas.borrow(),
                state.resolved_display.borrow().as_ref(),
                job,
            )
        {
            job.origin = origin;
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        state
            .copy_authority
            .replace(cancel.clone())
            .store(true, Ordering::Release);
        job.cancel.store(true, Ordering::Release);
        job.next = Some(Next {
            stamp: super::display_snapshot::Stamp::capture(
                &state.canvas.borrow(),
                state.resolved_display.borrow().as_ref(),
            ),
            plan,
            sequence,
            origin,
            cancel,
        });
        return;
    }
    if SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        report(hwnd, crate::ui_text::text("action-copy-unavailable"));
        return;
    }
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
                job.origin = origin;
                // Cancel old workers across windows; original windows still reclaim old results, without HWND/State transfer.
                state
                    .copy_authority
                    .replace(job.cancel.clone())
                    .store(true, Ordering::Release);
            } else {
                let _ = KillTimer(Some(hwnd), TIMER);
            }
            *state.copy_job.borrow_mut() = job;
        }
        Err(error) => {
            let _ = KillTimer(Some(hwnd), TIMER);
            report(hwnd, &error);
        }
    }
}
pub(super) fn cancel(state: &State) {
    if let Some(job) = state.copy_job.borrow().as_ref() {
        job.cancel();
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

// Tests replace only the final external side effect; reclamation, cancellation, and document validation fully share production paths.
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
    if let Some(next) = job.next.take() {
        drop(result);
        // Continue only content still belonging to this keystroke; never overwrite later edits, other copies, or cancellation.
        if IsWindow(Some(hwnd)).as_bool()
            && !state.modals.blocked()
            && !next.cancel.load(Ordering::Acquire)
            && !next
                .sequence
                .is_some_and(|expected| current_sequence() != expected)
            && gesture_idle(&state.canvas.borrow())
            && next.stamp.matches(
                &state.canvas.borrow(),
                state.resolved_display.borrow().as_ref(),
            )
        {
            start_guarded_at(hwnd, state, next.plan, next.sequence, next.origin);
        }
        return;
    }
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
                content.origin = job.origin.clone();
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
            // After publication begins, failure may follow partial format transfer; retain backing files and never delete them here.
            match publish(hwnd, &ready.payload, &paths, job.sequence) {
                Ok(Some(sequence)) => state.editable_clipboard.borrow_mut().published_at(
                    sequence,
                    if job.preserve_edits {
                        ready.editable
                    } else {
                        None
                    },
                    job.origin.take(),
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
        &HSTRING::from(crate::ui_text::format(
            "action-copy-failed",
            &[("error", message.into())],
        )),
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
