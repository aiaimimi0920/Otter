//! 模糊缩放只排有界的轻量输入，逐项复用原 worker/历史；目标或手势换代后不再续做。
use crate::{
    desktop::{self, canvas::Canvas, State},
    editor::{DocumentStamp, EditError, Tool},
    viewport::Point,
};
use std::collections::VecDeque;
use windows::Win32::Foundation::HWND;

// 每项仅含原始 delta 和中心模式，最多约 16 KiB；不保存图像/候选或创建等量任务。
const LIMIT: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Intent {
    delta: i16,
    own: bool,
}
struct Batch {
    stamp: DocumentStamp,
    indices: Vec<usize>,
    pending: VecDeque<Intent>,
}
#[derive(Default)]
pub(in crate::desktop) struct Session(Option<Batch>);
impl Session {
    fn begin(&mut self, canvas: &Canvas) {
        self.0 = Some(Batch {
            stamp: canvas.editor.document_stamp(),
            indices: canvas.selected_indices(),
            pending: VecDeque::new(),
        });
    }
    fn matches(&self, canvas: &Canvas) -> bool {
        self.0.as_ref().is_some_and(|batch| {
            eligible(canvas)
                && batch.stamp.matches(&canvas.editor)
                && batch.indices.as_slice() == canvas.selected_indices_ref()
        })
    }
    fn push(&mut self, intent: Intent) -> Result<(), EditError> {
        let batch = self
            .0
            .as_mut()
            .expect("wheel input must have a batch owner");
        if batch.pending.len() == LIMIT {
            return Err(EditError::BudgetExceeded);
        }
        batch.pending.push_back(intent);
        Ok(())
    }
}

fn available(state: &State) -> bool {
    !state.session_end.paused()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && desktop::capture_input::exclusive_available(state)
        && state.open_job.borrow().is_none()
        && !state.blur_job.borrow().is_busy()
        && !state.erase_job.borrow().is_busy()
        && !state.flip_job.borrow().is_busy()
        && state.crop_job.borrow().is_none()
        && state.rasterize_job.borrow().is_none()
}

pub(super) fn clear(state: &State) {
    state.resize_wheel.borrow_mut().0 = None;
}
fn cancel(state: &State) {
    if state.resize_wheel.borrow_mut().0.take().is_some() {
        state.resize_job.borrow().cancel();
    }
}
pub(in crate::desktop) unsafe fn dispatch(hwnd: HWND, state: &State, delta: i16, own: bool) {
    if delta == 0 {
        return;
    }
    if !available(state) || !eligible(&state.canvas.borrow()) {
        cancel(state);
        return;
    }
    if state.resize_wheel.borrow().0.is_some() {
        if !state.resize_wheel.borrow().matches(&state.canvas.borrow()) {
            cancel(state);
            return;
        }
        if let Err(error) = state.resize_wheel.borrow_mut().push(Intent { delta, own }) {
            eprintln!("native-annotation-wheel: {error:?}; new input rejected");
        }
        resume(hwnd, state);
        return;
    }
    // 只有本入口拥有的 resize worker 可以接续；不能给菜单缩放或其它编辑任务追加输入。
    if state.resize_job.borrow().is_busy() || !desktop::display_edit::prepare(state) {
        return;
    }
    submit(hwnd, state, Intent { delta, own });
}

unsafe fn submit(hwnd: HWND, state: &State, intent: Intent) {
    let result = prepare(&mut state.canvas.borrow_mut(), intent.delta, intent.own);
    match result {
        Ok(true) => {
            if state
                .canvas
                .borrow()
                .scale_pending
                .as_ref()
                .unwrap()
                .strokes()
                .any(|(_, stroke)| stroke.tool == Tool::Blur)
                && state.resize_wheel.borrow().0.is_none()
            {
                state
                    .resize_wheel
                    .borrow_mut()
                    .begin(&state.canvas.borrow());
            }
            super::submit_pending(hwnd, state);
            if !state.resize_job.borrow().is_busy() {
                clear(state);
            }
        }
        Ok(false) => clear(state),
        Err(error) => {
            clear(state);
            eprintln!("native-annotation-wheel: {error:?}; document unchanged");
        }
    }
}

pub(super) fn permit_completion(state: &State) -> bool {
    if state.resize_wheel.borrow().0.is_none() {
        return true;
    }
    let valid = available(state) && state.resize_wheel.borrow().matches(&state.canvas.borrow());
    if !valid {
        clear(state);
    }
    valid
}
pub(super) fn committed(state: &State) {
    // 先更新凭证再触发 Win32 重绘，避免同步重入把本次合法 revision 当成外部换代。
    let canvas = state.canvas.borrow();
    let mut session = state.resize_wheel.borrow_mut();
    if let Some(batch) = session.0.as_mut() {
        batch.stamp = canvas.editor.document_stamp();
        if batch.pending.is_empty() {
            session.0 = None;
        }
    }
}
pub(super) unsafe fn resume(hwnd: HWND, state: &State) {
    if state.resize_job.borrow().is_busy() || state.resize_wheel.borrow().0.is_none() {
        return;
    }
    if !permit_completion(state) {
        return;
    }
    let next = state
        .resize_wheel
        .borrow_mut()
        .0
        .as_mut()
        .unwrap()
        .pending
        .pop_front();
    if let Some(intent) = next {
        submit(hwnd, state, intent);
    } else {
        clear(state);
    }
}

fn prepare(canvas: &mut Canvas, delta: i16, own: bool) -> Result<bool, EditError> {
    if delta == 0 || !eligible(canvas) {
        return Ok(false);
    }
    let indices = canvas.selected_indices_ref();
    let Some(pivot) =
        crate::group_bounds::center(indices.iter().map(|i| &canvas.editor.elements()[*i]))
    else {
        return Ok(false);
    };
    let factor = (f32::from(delta) * 0.001).exp().clamp(0.5, 1.5);
    canvas.scale_pending = canvas.editor.prepare_resize_many_mode(
        indices,
        pivot,
        Point {
            x: factor,
            y: factor,
        },
        own,
        |text| {
            unsafe { desktop::annotation_text::measure(text) }.map_err(|_| EditError::InvalidPoint)
        },
    )?;
    Ok(canvas.scale_pending.is_some())
}
fn eligible(canvas: &Canvas) -> bool {
    canvas.pinned
        && canvas.minified.is_none()
        && canvas.tool == Tool::Select
        && !canvas.crop_mode
        && !canvas.blocks_wheel()
        && canvas.scale_pending.is_none()
        && !canvas.selected_indices_ref().is_empty()
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "procedure_wheel_tests.rs"]
mod tests;
