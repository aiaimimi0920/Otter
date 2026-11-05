//! 应用级后台通知：同一时刻最多排队一个线程消息，不启动空闲定时器。
use std::sync::{Arc, Mutex};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP},
};

const MESSAGE: u32 = WM_APP + 92;
struct State {
    active: bool,
    pending: bool,
}
pub(in crate::desktop) struct Wake {
    state: Arc<Mutex<State>>,
    thread: u32,
}
impl Wake {
    /// 调用方必须已创建窗口，使当前 UI 线程的消息队列存在。
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                active: true,
                pending: false,
            })),
            thread: unsafe { GetCurrentThreadId() },
        }
    }
    pub(in crate::desktop) fn callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        let state = self.state.clone();
        let thread = self.thread;
        Arc::new(move || {
            post(&state, || unsafe {
                PostThreadMessageW(thread, MESSAGE, WPARAM(0), LPARAM(0)).is_ok()
            });
        })
    }
    /// 必须先清 pending 再读取任务邮箱；并发更新由本次读取或下一条消息覆盖。
    pub(super) fn acknowledge(&self) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).pending = false;
    }
    pub(super) fn close(&self) {
        // 与投递互斥，close 返回后即使线程 ID 复用，旧 callback 也不能再投递。
        self.state.lock().unwrap_or_else(|e| e.into_inner()).active = false;
    }
}
impl Drop for Wake {
    fn drop(&mut self) {
        self.close();
    }
}
fn post(state: &Mutex<State>, send: impl FnOnce() -> bool) {
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    if state.active && !state.pending {
        // 投递失败不占 pending，后续通知可重试；队列满时已有消息驱动应用轮询。
        state.pending = send();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_retries_and_disables_old_callbacks() {
        let state = Mutex::new(State {
            active: true,
            pending: false,
        });
        post(&state, || false);
        assert!(!state.lock().unwrap().pending);
        post(&state, || true);
        post(&state, || panic!("duplicate wake"));
        state.lock().unwrap().pending = false;
        post(&state, || true);
        state.lock().unwrap().active = false;
        state.lock().unwrap().pending = false;
        post(&state, || panic!("closed wake"));
    }
    #[test]
    fn worker_wakes_real_ui_queue_without_input() {
        use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_NOREMOVE, PM_REMOVE};
        let mut message = MSG::default();
        unsafe {
            let _ = PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_NOREMOVE);
        }
        let wake = Wake::new();
        let callback = wake.callback();
        std::thread::spawn(move || {
            for _ in 0..100 {
                callback();
            }
        })
        .join()
        .unwrap();
        assert!(unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool());
        assert!(
            !unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool()
        );
        wake.acknowledge();
        wake.callback()();
        assert!(unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool());
        let late = wake.callback();
        drop(wake);
        late();
        assert!(
            !unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool()
        );
    }
}
