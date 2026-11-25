//! B 的本地输入法定位：源 caret 优先、点击后备；组合和已提交字符仍走原系统/队列。
use super::*;
use crate::live_window_target::input_routing::caret::Observation;
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::Ime::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Anchor {
    click: Option<[f64; 2]>,
    caret: Option<Observation>,
    sampled: Option<Instant>,
    revision: u64,
    error: Option<HRESULT>,
}
impl Anchor {
    pub(super) fn clear(&mut self) {
        *self = Self {
            revision: self.revision.wrapping_add(1),
            ..Default::default()
        };
    }
    pub(super) fn click(&mut self, point: [f64; 2]) {
        self.clear();
        self.click = Some(point);
    }
    fn due(&mut self, now: Instant) -> Option<u64> {
        if self
            .sampled
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(16))
        {
            return None;
        }
        self.sampled = Some(now);
        Some(self.revision)
    }
    fn observe(&mut self, revision: u64, result: &Result<Option<Observation>>) -> bool {
        if self.revision != revision {
            return false; // 只读 Win32 查询若重入暂停/新点击，不能重新挂回旧观测。
        }
        self.caret = result.as_ref().ok().copied().flatten();
        let error = result.as_ref().err().map(Error::code);
        let report = error.is_some() && error != self.error;
        self.error = error;
        report
    }
    fn point(&self, view: crate::live_viewport::Viewport) -> Option<[i32; 2]> {
        // 已知源 caret 不在当前裁剪内时不退回旧点击；无系统 caret 才使用点击后备。
        let source = if let Some(caret) = self.caret {
            if caret.size != view.source_size() {
                return None; // 源 resize 的新几何不能套到 B 尚未重新提交的旧纹理上。
            }
            caret.point
        } else {
            self.click?
        };
        let point = view.project(source)?;
        Some(point.map(|p| p.round() as i32))
    }
    unsafe fn place(&self, hwnd: HWND, view: crate::live_viewport::Viewport) -> Result<()> {
        let Some([x, y]) = self.point(view) else {
            return Ok(());
        };
        // IMM 的位置设置可能同步产生通知；不在重入中递归设置同一输入法上下文。
        if PLACING.with(|placing| placing.replace(true)) {
            return Ok(());
        }
        let _placing = Placing;
        let context = ImmGetContext(hwnd);
        if context.0.is_null() {
            return Ok(());
        }
        struct Context(HWND, HIMC);
        impl Drop for Context {
            fn drop(&mut self) {
                unsafe {
                    let _ = ImmReleaseContext(self.0, self.1);
                }
            }
        }
        let _context = Context(hwnd, context);
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        let point = POINT {
            x: x.clamp(0, (client.right - 1).max(0)),
            y: y.clamp(0, (client.bottom - 1).max(0)),
        };
        if !ImmSetCompositionWindow(
            context,
            &COMPOSITIONFORM {
                dwStyle: CFS_POINT,
                ptCurrentPos: point,
                rcArea: client,
            },
        )
        .as_bool()
        {
            return Err(Error::new(E_FAIL, "Live IME composition position failed"));
        }
        if !ImmSetCandidateWindow(
            context,
            &CANDIDATEFORM {
                dwIndex: 0,
                dwStyle: CFS_CANDIDATEPOS,
                ptCurrentPos: point,
                rcArea: client,
            },
        )
        .as_bool()
        {
            return Err(Error::new(E_FAIL, "Live IME candidate position failed"));
        }
        Ok(())
    }
}

thread_local! {
    static PLACING: Cell<bool> = const { Cell::new(false) };
}
struct Placing;
impl Drop for Placing {
    fn drop(&mut self) {
        PLACING.with(|placing| placing.set(false));
    }
}

/// 只使用 poll 已采样的结果；WndProc 不跨进程查询源，不跨 IMM 调用持有 RefCell。
pub(super) unsafe fn handle(hwnd: HWND, state: &State, message: u32, wp: WPARAM) -> bool {
    if !matches!(
        message,
        WM_IME_STARTCOMPOSITION | WM_IME_COMPOSITION | WM_IME_NOTIFY
    ) {
        return false;
    }
    let reposition =
        message != WM_IME_NOTIFY || matches!(wp.0 as u32, IMN_OPENCANDIDATE | IMN_CHANGECANDIDATE);
    if reposition
        && state.input.borrow().enabled
        && windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() == hwnd
    {
        let anchor = state.input.borrow().ime;
        if let Some(view) = super::viewport(hwnd, state) {
            if let Err(error) = anchor.place(hwnd, view) {
                eprintln!("native-live-ime-position: {error}");
            }
        }
    }
    true // DefWindowProc 保有组合/候选 UI；WM_IME_CHAR 最终生成 WM_CHAR。
}

impl LiveWindow {
    pub(crate) fn refresh_ime(
        &self,
        target: Option<&crate::live_window_target::Target>,
        crop: Option<crate::live_window_target::region::Crop>,
    ) {
        let revision = {
            let mut input = self.state.input.borrow_mut();
            if !self.is_alive()
                || input.remote
                || input.failed
                || self.state.group_hidden.get()
                || self.state.group_locked.get()
                || unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() } != self.hwnd
                || target.is_none()
            {
                input.ime.clear();
                return;
            }
            if !input.enabled {
                // 鼠标首击可能正在等待 worker Enable；不能丢掉同一 FIFO 手势的后备锚点。
                if !input.pending {
                    input.ime.clear();
                }
                return;
            }
            input.ime.due(Instant::now())
        };
        let Some(revision) = revision else { return };
        let result = crate::live_window_target::input_routing::caret::sample(target.unwrap(), crop);
        let report = self.state.input.borrow_mut().ime.observe(revision, &result);
        if report {
            if let Err(error) = result {
                // 定位失败不能伪装成控制失败；清除旧 caret 并使用原点击后备，按错误码去重。
                eprintln!("native-live-ime-caret: {error}");
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_ime_tests.rs"]
mod tests;
